#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::{transmute, ManuallyDrop};
use std::rc::Rc;

#[cfg(not(target_os = "ios"))]
use core_graphics::context::CGContext;
use flexbuffers::DeserializationError;
#[cfg(target_os = "ios")]
use pax_pixels::{
    render_backend::{RenderBackend, RenderConfig},
    point, Box2D, Image as PaxPixelsImage, WgpuRenderer,
};
use pax_runtime::api::math::Point2;
#[cfg(target_os = "ios")]
use pax_runtime::api::Axis;
use pax_runtime::api::{
    ButtonClick, Click, Event, Focus, ModifierKey, MouseButton, MouseEventArgs, RenderContext,
    SelectStart, TextboxChange,
};
#[cfg(target_os = "ios")]
use pax_runtime::pax_pixels_render_context::{convert_kurbo_to_lyon_path, to_pax_pixels_color};
use pax_runtime::PaxEngine;
use piet::kurbo;
use piet::kurbo::Shape;
#[cfg(not(target_os = "ios"))]
use piet::{InterpolationMode, RenderContext as PietRenderContext};
#[cfg(not(target_os = "ios"))]
use piet_coregraphics::CoreGraphicsContext;
use serde::Serialize;

//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `PaxCartridge.h`
//in order to be visible to Swift
pub use pax_message::*;

#[cfg(not(target_os = "ios"))]
struct ImgData<'a> {
    img: <CoreGraphicsContext<'a> as PietRenderContext>::Image,
    size: (usize, usize),
}

#[cfg(not(target_os = "ios"))]
struct AppleRenderContext<'a> {
    backend: CoreGraphicsContext<'a>,
    image_map: HashMap<String, ImgData<'a>>,
}

#[cfg(not(target_os = "ios"))]
impl<'a> AppleRenderContext<'a> {
    fn new(backend: CoreGraphicsContext<'a>) -> Self {
        Self {
            backend,
            image_map: HashMap::new(),
        }
    }
}

#[cfg(not(target_os = "ios"))]
impl<'a> RenderContext for AppleRenderContext<'a> {
    fn fill(&mut self, _layer: usize, path: kurbo::BezPath, brush: &pax_runtime::api::Fill) {
        self.backend
            .fill(path.clone(), &fill_to_piet_brush(brush, path.bounding_box()));
    }

    fn stroke(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        brush: &pax_runtime::api::Fill,
        width: f64,
    ) {
        self.backend
            .stroke(path.clone(), &fill_to_piet_brush(brush, path.bounding_box()), width);
    }

    fn save(&mut self, _layer: usize) {
        let _ = self.backend.save();
    }

    fn restore(&mut self, _layer: usize) {
        let _ = self.backend.restore();
    }

    fn clip(&mut self, _layer: usize, path: kurbo::BezPath) {
        self.backend.clip(path);
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        let img = self
            .backend
            .make_image(width, height, buf, piet::ImageFormat::RgbaSeparate)
            .expect("image creation successful");
        self.image_map.insert(
            path.to_owned(),
            ImgData {
                img,
                size: (width, height),
            },
        );
    }

    fn draw_image(&mut self, _layer: usize, image_path: &str, rect: kurbo::Rect) {
        let Some(data) = self.image_map.get(image_path) else {
            return;
        };
        self.backend
            .draw_image(&data.img, rect, InterpolationMode::Bilinear);
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map.get(image_path).map(|img| img.size)
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        self.image_map.contains_key(image_path)
    }

    fn transform(&mut self, _layer: usize, affine: kurbo::Affine) {
        self.backend.transform(affine);
    }

    fn layers(&self) -> usize {
        1
    }

    fn resize_layers_to(&mut self, _layer_count: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {}

    fn clear(&mut self, _layer: usize) {}

    fn flush(&mut self, _layer: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {}

    fn resize(&mut self, _width: usize, _height: usize) {}
}

#[cfg(not(target_os = "ios"))]
fn fill_to_piet_brush(fill: &pax_runtime::api::Fill, rect: kurbo::Rect) -> piet::PaintBrush {
    use piet::{LinearGradient, RadialGradient};

    match fill {
        pax_runtime::api::Fill::Solid(color) => color.to_piet_color().into(),
        pax_runtime::api::Fill::LinearGradient(linear) => {
            let linear_gradient = LinearGradient::new(
                pax_runtime::api::Fill::to_unit_point(linear.start, (rect.width(), rect.height())),
                pax_runtime::api::Fill::to_unit_point(linear.end, (rect.width(), rect.height())),
                pax_runtime::api::Fill::to_piet_gradient_stops(linear.stops.clone()),
            );
            linear_gradient.into()
        }
        pax_runtime::api::Fill::RadialGradient(radial) => {
            let origin =
                pax_runtime::api::Fill::to_unit_point(radial.start, (rect.width(), rect.height()));
            let center =
                pax_runtime::api::Fill::to_unit_point(radial.end, (rect.width(), rect.height()));
            let radial_gradient =
                RadialGradient::new(radial.radius, pax_runtime::api::Fill::to_piet_gradient_stops(radial.stops.clone()))
                    .with_center(center)
                    .with_origin(origin);
            radial_gradient.into()
        }
    }
}

#[cfg(target_os = "ios")]
pub struct AppleRenderContext {
    backend: WgpuRenderer<'static>,
    image_map: HashMap<String, PaxPixelsImage>,
    image_versions: HashMap<String, u64>,
    logical_size: (usize, usize),
    dpr: u32,
}

#[cfg(target_os = "ios")]
impl AppleRenderContext {
    fn new(layer: *mut c_void, width: usize, height: usize, dpr: f32) -> Result<Self, String> {
        let dpr = dpr.round().max(1.0) as u32;
        let config = RenderConfig::new(false, width as u32, height as u32, dpr);
        let backend = unsafe {
            pollster::block_on(RenderBackend::to_core_animation_layer(layer, config))
        }
        .map_err(|err| err.to_string())?;
        let mut backend = WgpuRenderer::new(backend);
        backend.resize_surface((width as f32 * dpr as f32).max(1.0), (height as f32 * dpr as f32).max(1.0));
        backend.set_viewport(width as f32, height as f32, dpr as f32);
        Ok(Self {
            backend,
            image_map: HashMap::new(),
            image_versions: HashMap::new(),
            logical_size: (width, height),
            dpr,
        })
    }

    fn resize_if_needed(&mut self, width: usize, height: usize, dpr: f32) -> bool {
        let dpr = dpr.round().max(1.0) as u32;
        if self.logical_size == (width, height) && self.dpr == dpr {
            return false;
        }
        self.backend
            .resize_surface((width as f32 * dpr as f32).max(1.0), (height as f32 * dpr as f32).max(1.0));
        self.backend.set_viewport(width as f32, height as f32, dpr as f32);
        self.logical_size = (width, height);
        self.dpr = dpr;
        true
    }
}

#[cfg(target_os = "ios")]
impl RenderContext for AppleRenderContext {
    fn fill(&mut self, _layer: usize, path: kurbo::BezPath, fill: &pax_runtime::api::Fill) {
        let bounds = path.bounding_box();
        self.backend.fill_path(
            convert_kurbo_to_lyon_path(&path),
            to_pax_pixels_fill(fill, bounds),
        );
    }

    fn stroke(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime::api::Fill,
        width: f64,
    ) {
        let bounds = path.bounding_box();
        self.backend.stroke_path(
            convert_kurbo_to_lyon_path(&path),
            to_pax_pixels_fill(fill, bounds),
            width as f32,
        );
    }

    fn save(&mut self, _layer: usize) {
        self.backend.save();
    }

    fn restore(&mut self, _layer: usize) {
        self.backend.restore();
    }

    fn clip(&mut self, _layer: usize, path: kurbo::BezPath) {
        self.backend.clip(convert_kurbo_to_lyon_path(&path));
    }

    fn transform(&mut self, _layer: usize, affine: kurbo::Affine) {
        self.backend.transform(pax_pixels::Transform2D::from_array(
            affine.as_coeffs().map(|value| value as f32),
        ));
    }

    fn load_image(&mut self, path: &str, buf: &[u8], width: usize, height: usize) {
        self.image_map.insert(
            path.to_string(),
            PaxPixelsImage {
                rgba: buf.to_vec(),
                pixel_width: width as u32,
                pixel_height: height as u32,
            },
        );
        *self.image_versions.entry(path.to_string()).or_insert(0) += 1;
    }

    fn draw_image(&mut self, _layer: usize, image_path: &str, rect: kurbo::Rect) {
        let Some(image) = self.image_map.get(image_path) else {
            return;
        };
        let version = *self.image_versions.get(image_path).unwrap_or(&0);
        self.backend.draw_image(
            image_path,
            version,
            image,
            Box2D {
                min: point(rect.x0 as f32, rect.y0 as f32),
                max: point(rect.x1 as f32, rect.y1 as f32),
            },
        );
    }

    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)> {
        self.image_map
            .get(image_path)
            .map(|image| (image.pixel_width as usize, image.pixel_height as usize))
    }

    fn image_loaded(&self, image_path: &str) -> bool {
        self.image_map.contains_key(image_path)
    }

    fn layers(&self) -> usize {
        1
    }

    fn resize_layers_to(&mut self, _layer_count: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {}

    fn clear(&mut self, _layer: usize) {
        self.backend.clear();
    }

    fn flush(&mut self, _layer: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        self.backend.flush();
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.backend.resize(width as f32, height as f32);
    }

    fn begin_node(&mut self, _layer: usize, node_id: u32, z_index: i32) -> bool {
        self.backend.begin_node(node_id, z_index)
    }

    fn end_node(&mut self, _layer: usize, node_id: u32) -> bool {
        self.backend.end_node(node_id)
    }

    fn remove_node(&mut self, _layer: usize, node_id: u32) -> bool {
        self.backend.remove_node(node_id)
    }
}

#[cfg(target_os = "ios")]
fn to_pax_pixels_fill(fill: &pax_runtime::api::Fill, rect: kurbo::Rect) -> pax_pixels::Fill {
    let bounds = (rect.width(), rect.height());
    let origin = rect.origin();
    match fill {
        pax_runtime::api::Fill::Solid(color) => {
            pax_pixels::Fill::Solid(to_pax_pixels_color(color))
        }
        pax_runtime::api::Fill::LinearGradient(gradient) => {
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let main_axis =
                pax_pixels::Vector2D::new((end_x - start_x) as f32, (end_y - start_y) as f32);
            pax_pixels::Fill::Gradient {
                stops: gradient
                    .stops
                    .iter()
                    .map(|stop| pax_pixels::GradientStop {
                        color: to_pax_pixels_color(&stop.color),
                        stop: stop
                            .position
                            .evaluate((main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
                gradient_type: pax_pixels::GradientType::Linear,
                pos: pax_pixels::Point2D::new(
                    (origin.x + start_x) as f32,
                    (origin.y + start_y) as f32,
                ),
                main_axis,
                off_axis: pax_pixels::Vector2D::zero(),
            }
        }
        pax_runtime::api::Fill::RadialGradient(gradient) => {
            let start_x = gradient.start.0.evaluate(bounds, Axis::X);
            let start_y = gradient.start.1.evaluate(bounds, Axis::Y);
            let end_x = gradient.end.0.evaluate(bounds, Axis::X);
            let end_y = gradient.end.1.evaluate(bounds, Axis::Y);
            let radius = gradient.radius as f32;
            let main_axis = pax_pixels::Vector2D::new(
                radius * (end_x - start_x) as f32,
                radius * (end_y - start_y) as f32,
            );
            let off_axis = pax_pixels::Vector2D::new(-main_axis.y, main_axis.x);
            pax_pixels::Fill::Gradient {
                gradient_type: pax_pixels::GradientType::Radial,
                pos: pax_pixels::Point2D::new(
                    (origin.x + start_x) as f32,
                    (origin.y + start_y) as f32,
                ),
                main_axis,
                off_axis,
                stops: gradient
                    .stops
                    .iter()
                    .map(|stop| pax_pixels::GradientStop {
                        color: to_pax_pixels_color(&stop.color),
                        stop: stop
                            .position
                            .evaluate((main_axis.length() as f64, 0.0), Axis::X)
                            as f32,
                    })
                    .collect(),
            }
        }
    }
}

fn serialize_message_queue(messages: Vec<NativeMessage>) -> *mut NativeMessageQueue {
    let wrapped_queue = MessageQueue { messages };
    let mut serializer = flexbuffers::FlexbufferSerializer::new();

    wrapped_queue.serialize(&mut serializer).unwrap();

    let data_buffer = serializer.take_buffer();
    let length = data_buffer.len();
    let leaked_data: ManuallyDrop<Box<[u8]>> = ManuallyDrop::new(data_buffer.into_boxed_slice());

    unsafe {
        transmute(Box::new(NativeMessageQueue {
            data_ptr: Box::into_raw(ManuallyDrop::into_inner(leaked_data)),
            length: length as u64,
        }))
    }
}

/// Container data structure for PaxEngine, aggregated to support passing across C bridge
#[repr(C)] //Exposed to Swift via PaxCartridge.h
pub struct PaxEngineContainer {
    pub _engine: *mut PaxEngine,
    #[cfg(target_os = "ios")]
    pub _render_context: *mut AppleRenderContext,
    #[cfg(target_os = "ios")]
    pub _render_target: *mut c_void,
}

/// Destroy `engine` and clean up the `ManuallyDrop` container surround it.
#[no_mangle]
pub extern "C" fn pax_dealloc_engine(container: *mut PaxEngineContainer) {
    if container.is_null() {
        return;
    }

    unsafe {
        let container = Box::from_raw(container);
        if !container._engine.is_null() {
            drop(Box::from_raw(container._engine));
        }
        #[cfg(target_os = "ios")]
        if !container._render_context.is_null() {
            drop(Box::from_raw(container._render_context));
        }
    }
}

/// Send `interrupt`s from the chassis, for example: user input
/// Note that in any single-threaded environment, these interrupts will happen
/// synchronously between engine ticks, allowing for safe unwrapping / borrowing
/// of engine and runtime here.
#[no_mangle]
pub extern "C" fn pax_interrupt(
    engine_container: *mut PaxEngineContainer,
    buffer: *const InterruptBuffer,
) {
    let engine = unsafe { Box::from_raw((*engine_container)._engine) };

    let length: u64 = unsafe { (*buffer).length.try_into().unwrap() };

    let slice = unsafe {
        if (*buffer).data_ptr.is_null() {
            &mut []
        } else {
            std::slice::from_raw_parts((*buffer).data_ptr, length.try_into().unwrap())
        }
    };

    let interrupt_wrapped: Result<NativeInterrupt, DeserializationError> = flexbuffers::from_slice(slice);
    let interrupt = interrupt_wrapped.unwrap();
    let globals = engine.runtime_context.globals();

    match &interrupt {
        NativeInterrupt::Focus(_args) => {
            engine.global_dispatch_focus(Focus {});
        }
        NativeInterrupt::SelectStart(_args) => {
            engine.global_dispatch_select_start(SelectStart {});
        }
        NativeInterrupt::ChassisResizeRequestCollection(collection) => {
            for args in collection {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    node.chassis_resize_request(args.width, args.height);
                }
            }
        }
        NativeInterrupt::Click(args) => {
            let topmost_node = engine
                .runtime_context
                .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
            let modifiers = args
                .modifiers
                .iter()
                .map(ModifierKey::from)
                .collect();
            let args_click = Click {
                mouse: MouseEventArgs {
                    x: args.x,
                    y: args.y,
                    button: MouseButton::from(args.button.clone()),
                    modifiers,
                },
            };
            topmost_node.dispatch_click(Event::new(args_click), &globals, &engine.runtime_context);
        }
        NativeInterrupt::FormRadioSetChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormSliderChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormDropdownChange(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormButtonClick(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                node.dispatch_button_click(
                    Event::new(ButtonClick {}),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::FormTextboxInput(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::TextInput(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormTextboxChange(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                node.dispatch_textbox_change(
                    Event::new(TextboxChange {
                        text: args.text.clone(),
                    }),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::FormCheckboxToggle(args) => {
            if let Some(node) = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::Scrollbar(_args) => {}
        NativeInterrupt::Scroll(_args) => {}
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(_ref_args) => {
                #[cfg(target_os = "ios")]
                {
                    let ref_args = _ref_args;
                    let Some(render_context) = (unsafe { (*engine_container)._render_context.as_mut() })
                    else {
                        unsafe { (*engine_container)._engine = Box::into_raw(engine) };
                        return;
                    };
                    let identifier = if ref_args.path.is_empty() {
                        format!("image-{}", ref_args.id)
                    } else {
                        ref_args.path.clone()
                    };
                    let image_data = unsafe {
                        std::slice::from_raw_parts(
                            ref_args.image_data as *const u8,
                            ref_args.image_data_length,
                        )
                    };
                    render_context.load_image(
                        &identifier,
                        image_data,
                        ref_args.width,
                        ref_args.height,
                    );
                    engine.runtime_context.set_all_canvases_dirty();
                }
            }
            ImageLoadInterruptArgs::Data(_args) => {}
        },
        NativeInterrupt::AddedLayer(_args) => {}
        _ => {}
    }

    unsafe { (*engine_container)._engine = Box::into_raw(engine) };
}

/// Perform full tick of engine, including property computation, lifecycle event handling, and rendering side-effects.
/// Returns a message queue of native rendering actions encoded as a Flexbuffer via FFI to Swift.
/// The returned message queue requires explicit deallocation: `pax_deallocate_message_queue`
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_tick(
    engine_container: *mut PaxEngineContainer,
    render_target: *mut c_void,
    width: f32,
    height: f32,
    _dpr: f32,
) -> *mut NativeMessageQueue {
    let mut engine = unsafe { Box::from_raw((*engine_container)._engine) };

    engine.set_viewport_size((width as f64, height as f64));
    let messages = engine.tick();

    #[cfg(target_os = "ios")]
    {
        if width > 0.0 && height > 0.0 && !render_target.is_null() {
            let container = unsafe { &mut *engine_container };
            let should_recreate =
                container._render_context.is_null() || container._render_target != render_target;

            if should_recreate {
                if !container._render_context.is_null() {
                    unsafe { drop(Box::from_raw(container._render_context)) };
                    container._render_context = std::ptr::null_mut();
                }
                match AppleRenderContext::new(render_target, width as usize, height as usize, _dpr) {
                    Ok(render_context) => {
                        container._render_context = Box::into_raw(Box::new(render_context));
                        container._render_target = render_target;
                        engine.runtime_context.set_all_canvases_dirty();
                    }
                    Err(err) => {
                        eprintln!("failed to initialize iOS gpu render context: {err}");
                    }
                }
            } else if let Some(render_context) = unsafe { container._render_context.as_mut() } {
                if render_context.resize_if_needed(width as usize, height as usize, _dpr) {
                    engine.runtime_context.set_all_canvases_dirty();
                }
            }

            let should_redraw_all = engine
                .runtime_context
                .dirty_canvases
                .borrow()
                .iter()
                .any(|dirty| *dirty);
            if should_redraw_all {
                engine.runtime_context.set_all_canvases_dirty();
            }

            if let Some(render_context) = unsafe { container._render_context.as_mut() } {
                engine.render(render_context as &mut dyn RenderContext);
            }
        }
    }

    #[cfg(not(target_os = "ios"))]
    {
        let will_cast_cgContext = render_target as *mut CGContext;
        let ctx = unsafe { &mut *will_cast_cgContext };
        let mut render_context =
            AppleRenderContext::new(CoreGraphicsContext::new_y_up(ctx, height as f64, None));
        engine.render(&mut render_context as &mut dyn RenderContext);
    }

    let queue_container = serialize_message_queue(messages);
    unsafe { (*engine_container)._engine = Box::into_raw(engine) };

    queue_container
}

/// Required manual cleanup callback from Swift after reading a frame's message queue.
/// If this is not called after `pax_tick` is invoked, we will have a memory leak.
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_dealloc_message_queue(queue: *mut NativeMessageQueue) {
    unsafe {
        let queue_container = Box::from_raw(queue);
        let data_buffer = Box::from_raw(queue_container.data_ptr);
        drop(data_buffer);
        drop(queue_container);
    }
}
