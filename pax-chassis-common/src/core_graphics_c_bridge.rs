#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::{transmute, ManuallyDrop};
use std::rc::Rc;

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use core_graphics::context::CGContext;
use flexbuffers::DeserializationError;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_pixels::{
    point,
    render_backend::{RenderBackend, RenderConfig},
    Box2D, Image as PaxPixelsImage, WgpuRenderer,
};
use pax_runtime::api::math::Point2;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_runtime::api::Axis;
use pax_runtime::api::{
    ButtonClick, Click, Event, Focus, ModifierKey, MouseButton, MouseEventArgs, RenderContext,
    SelectStart, TextboxChange, Touch, TouchEnd, TouchMove, TouchStart,
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_runtime::pax_pixels_render_context::{convert_kurbo_to_lyon_path, to_pax_pixels_color};
use pax_runtime::PaxEngine;
use piet::kurbo;
use piet::kurbo::Shape;
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet::{InterpolationMode, RenderContext as PietRenderContext};
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet_coregraphics::CoreGraphicsContext;
use serde::Deserialize;
use serde::Serialize;

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;
#[cfg(feature = "designtime")]
use pax_runtime::designtime_support::{
    apply_designtime_replace_node_subtemplate, apply_designtime_userland_reload,
    build_designtime_inspect_tree_payload, build_designtime_ray_cast_payload,
    build_designtime_selector_query_payload,
};
use pax_runtime::DefinitionToInstanceTraverser;
//Re-export all native message types; used by Swift via FFI.
//Note that any types exposed by pax_message must ALSO be added to `PaxCartridge.h`
//in order to be visible to Swift
pub use pax_message::*;

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
struct ImgData<'a> {
    img: <CoreGraphicsContext<'a> as PietRenderContext>::Image,
    size: (usize, usize),
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
struct AppleRenderContext<'a> {
    backend: CoreGraphicsContext<'a>,
    image_map: HashMap<String, ImgData<'a>>,
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
impl<'a> AppleRenderContext<'a> {
    fn new(backend: CoreGraphicsContext<'a>) -> Self {
        Self {
            backend,
            image_map: HashMap::new(),
        }
    }
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
impl<'a> RenderContext for AppleRenderContext<'a> {
    fn fill(&mut self, _layer: usize, path: kurbo::BezPath, brush: &pax_runtime::api::Fill) {
        self.backend.fill(
            path.clone(),
            &fill_to_piet_brush(brush, path.bounding_box()),
        );
    }

    fn stroke(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        brush: &pax_runtime::api::Fill,
        width: f64,
    ) {
        self.backend.stroke(
            path.clone(),
            &fill_to_piet_brush(brush, path.bounding_box()),
            width,
        );
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

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
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
            let radial_gradient = RadialGradient::new(
                radial.radius,
                pax_runtime::api::Fill::to_piet_gradient_stops(radial.stops.clone()),
            )
            .with_center(center)
            .with_origin(origin);
            radial_gradient.into()
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub struct AppleRenderContext {
    backend: WgpuRenderer<'static>,
    image_map: HashMap<String, PaxPixelsImage>,
    image_versions: HashMap<String, u64>,
    logical_size: (usize, usize),
    dpr: u32,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl AppleRenderContext {
    fn new(layer: *mut c_void, width: usize, height: usize, dpr: f32) -> Result<Self, String> {
        let dpr = dpr.round().max(1.0) as u32;
        let config = RenderConfig::new(false, width as u32, height as u32, dpr as f32);
        let backend =
            unsafe { pollster::block_on(RenderBackend::to_core_animation_layer(layer, config)) }
                .map_err(|err| err.to_string())?;
        let mut backend = WgpuRenderer::new(backend);
        backend.resize_surface(
            (width as f32 * dpr as f32).max(1.0),
            (height as f32 * dpr as f32).max(1.0),
        );
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
        self.backend.resize_surface(
            (width as f32 * dpr as f32).max(1.0),
            (height as f32 * dpr as f32).max(1.0),
        );
        self.backend
            .set_viewport(width as f32, height as f32, dpr as f32);
        self.logical_size = (width, height);
        self.dpr = dpr;
        true
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl RenderContext for AppleRenderContext {
    fn fill_with_opacity(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime::api::Fill,
        opacity: f64,
    ) {
        let bounds = path.bounding_box();
        self.backend.fill_path_with_opacity(
            convert_kurbo_to_lyon_path(&path),
            to_pax_pixels_fill(fill, bounds),
            opacity as f32,
        );
    }

    fn stroke_with_opacity(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        fill: &pax_runtime::api::Fill,
        width: f64,
        opacity: f64,
    ) {
        let bounds = path.bounding_box();
        self.backend.stroke_path_with_opacity(
            convert_kurbo_to_lyon_path(&path),
            to_pax_pixels_fill(fill, bounds),
            width as f32,
            opacity as f32,
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

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn to_pax_pixels_fill(fill: &pax_runtime::api::Fill, rect: kurbo::Rect) -> pax_pixels::Fill {
    let bounds = (rect.width(), rect.height());
    let origin = rect.origin();
    match fill {
        pax_runtime::api::Fill::Solid(color) => pax_pixels::Fill::Solid(to_pax_pixels_color(color)),
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
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub _render_context: *mut AppleRenderContext,
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    pub _render_target: *mut c_void,
    #[cfg(feature = "designtime")]
    pub userland_definition_to_instance_traverser:
        Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>,
    #[cfg(feature = "designtime")]
    pub designtime_manager: Rc<RefCell<DesigntimeManager>>,
}

#[derive(Serialize)]
struct InspectTreePayload {
    status: String,
    node_count: Option<usize>,
    tree_json: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RayCastRequestPayload {
    x: f64,
    y: f64,
    hit_invisible: bool,
}

#[derive(Serialize)]
struct InspectNodeListPayload {
    status: String,
    node_count: Option<usize>,
    nodes_json: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct SelectorQueryRequestPayload {
    selector: String,
}

#[derive(Deserialize)]
struct ReplaceNodeRequestPayload {
    component_type_id: String,
    template_node_id: usize,
    subtemplate: String,
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
        #[cfg(any(target_os = "ios", target_os = "macos"))]
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
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };

    let length: u64 = unsafe { (*buffer).length.try_into().unwrap() };

    let slice = unsafe {
        if (*buffer).data_ptr.is_null() {
            &mut []
        } else {
            std::slice::from_raw_parts((*buffer).data_ptr, length.try_into().unwrap())
        }
    };

    let interrupt_wrapped: Result<NativeInterrupt, DeserializationError> =
        flexbuffers::from_slice(slice);
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
            let modifiers = args.modifiers.iter().map(ModifierKey::from).collect();
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
        NativeInterrupt::TouchStart(args) => {
            if let Some(first_touch) = args.touches.first() {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(Touch::from).collect();
                topmost_node.dispatch_touch_start(
                    Event::new(TouchStart { touches }),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::TouchMove(args) => {
            if let Some(first_touch) = args.touches.first() {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(Touch::from).collect();
                topmost_node.dispatch_touch_move(
                    Event::new(TouchMove { touches }),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::TouchEnd(args) => {
            if let Some(first_touch) = args.touches.first() {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(Touch::from).collect();
                topmost_node.dispatch_touch_end(
                    Event::new(TouchEnd { touches }),
                    &globals,
                    &engine.runtime_context,
                );
            }
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
            if let Some(node) =
                engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                node.dispatch_button_click(
                    Event::new(ButtonClick {}),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::FormTextboxInput(args) => {
            if let Some(node) =
                engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::TextInput(args) => {
            if let Some(node) =
                engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::FormTextboxChange(args) => {
            if let Some(node) =
                engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
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
            if let Some(node) =
                engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
            {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::Scrollbar(_args) => {}
        NativeInterrupt::Scroll(_args) => {}
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(_ref_args) => {
                #[cfg(any(target_os = "ios", target_os = "macos"))]
                {
                    let ref_args = _ref_args;
                    let Some(render_context) =
                        (unsafe { (*engine_container)._render_context.as_mut() })
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
                    engine.runtime_context.mark_all_canvas_nodes_dirty();
                }
            }
            ImageLoadInterruptArgs::Data(_args) => {}
        },
        NativeInterrupt::AddedLayer(_args) => {}
        NativeInterrupt::Screenshot(args) => match args {
            ImageLoadInterruptArgs::Reference(ref_args) => {
                let ptr = ref_args.image_data as *const u8;
                let data =
                    unsafe { std::slice::from_raw_parts(ptr, ref_args.image_data_length).to_vec() };
                let screenshot_data = ScreenshotData {
                    id: ref_args.id,
                    data,
                    width: ref_args.width,
                    height: ref_args.height,
                };
                engine
                    .runtime_context
                    .load_screenshot(ref_args.id, screenshot_data);
            }
            ImageLoadInterruptArgs::Data(_data_args) => {}
        },
        _ => {}
    }

    unsafe { (*engine_container)._engine = Box::into_raw(engine) };
    let _ = Box::into_raw(engine_container);
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
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let mut engine = unsafe { Box::from_raw(engine_container._engine) };

    #[cfg(feature = "designtime")]
    {
        engine_container
            .designtime_manager
            .borrow_mut()
            .handle_recv(engine.runtime_context.get_screenshot_map())
            .unwrap_or_else(|err| eprintln!("designtime receive failed: {err:?}"));
        apply_designtime_userland_reload(
            &mut engine,
            engine_container
                .userland_definition_to_instance_traverser
                .as_ref(),
            &engine_container.designtime_manager,
        );
    }

    engine.set_viewport_size((width as f64, height as f64));
    let messages = engine.tick();

    #[cfg(any(target_os = "ios", target_os = "macos"))]
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
                match AppleRenderContext::new(render_target, width as usize, height as usize, _dpr)
                {
                    Ok(render_context) => {
                        container._render_context = Box::into_raw(Box::new(render_context));
                        container._render_target = render_target;
                        engine.runtime_context.set_all_canvases_dirty();
                        engine.runtime_context.mark_all_canvas_nodes_dirty();
                    }
                    Err(err) => {
                        eprintln!("failed to initialize Apple gpu render context: {err}");
                    }
                }
            } else if let Some(render_context) = unsafe { container._render_context.as_mut() } {
                if render_context.resize_if_needed(width as usize, height as usize, _dpr) {
                    engine.runtime_context.set_all_canvases_dirty();
                    engine.runtime_context.mark_all_canvas_nodes_dirty();
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
                engine.runtime_context.mark_all_canvas_nodes_dirty();
            }

            if let Some(render_context) = unsafe { container._render_context.as_mut() } {
                engine.render(render_context as &mut dyn RenderContext);
            }
        }
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let will_cast_cgContext = render_target as *mut CGContext;
        let ctx = unsafe { &mut *will_cast_cgContext };
        let mut render_context =
            AppleRenderContext::new(CoreGraphicsContext::new_y_up(ctx, height as f64, None));
        engine.render(&mut render_context as &mut dyn RenderContext);
    }

    let queue_container = serialize_message_queue(messages);
    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);
    queue_container
}

#[no_mangle]
pub extern "C" fn pax_designtime_inspect_tree(
    engine_container: *mut PaxEngineContainer,
    max_depth: i64,
) -> *mut NativeMessageQueue {
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };

    let payload = inspect_tree_payload(&engine_container, &engine, max_depth);
    let payload_bytes = serde_json::to_vec(&payload).unwrap_or_else(|err| {
        format!(
            "{{\"status\":\"error\",\"node_count\":null,\"tree_json\":null,\"error\":\"failed to serialize inspect tree payload: {}\"}}",
            err
        )
        .into_bytes()
    });

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);

    bytes_to_native_message_queue(payload_bytes)
}

#[no_mangle]
pub extern "C" fn pax_designtime_replace_node(
    engine_container: *mut PaxEngineContainer,
    request_buffer: *const InterruptBuffer,
) -> *mut NativeMessageQueue {
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };

    let payload_bytes = replace_node_payload_bytes(&engine_container, request_buffer);

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);

    bytes_to_native_message_queue(payload_bytes)
}

#[no_mangle]
pub extern "C" fn pax_designtime_ray_cast(
    engine_container: *mut PaxEngineContainer,
    request_buffer: *const InterruptBuffer,
) -> *mut NativeMessageQueue {
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };

    let payload_bytes = ray_cast_payload_bytes(&engine_container, &engine, request_buffer);

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);

    bytes_to_native_message_queue(payload_bytes)
}

#[no_mangle]
pub extern "C" fn pax_designtime_selector_query(
    engine_container: *mut PaxEngineContainer,
    request_buffer: *const InterruptBuffer,
) -> *mut NativeMessageQueue {
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };

    let payload_bytes = selector_query_payload_bytes(&engine_container, &engine, request_buffer);

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);

    bytes_to_native_message_queue(payload_bytes)
}

fn bytes_to_native_message_queue(data_buffer: Vec<u8>) -> *mut NativeMessageQueue {
    let length = data_buffer.len();
    let leaked_data: ManuallyDrop<Box<[u8]>> = ManuallyDrop::new(data_buffer.into_boxed_slice());

    unsafe {
        transmute(Box::new(NativeMessageQueue {
            data_ptr: Box::into_raw(ManuallyDrop::into_inner(leaked_data)),
            length: length as u64,
        }))
    }
}

fn inspect_tree_payload(
    #[allow(unused_variables)] engine_container: &PaxEngineContainer,
    #[allow(unused_variables)] engine: &PaxEngine,
    #[allow(unused_variables)] max_depth: i64,
) -> InspectTreePayload {
    #[cfg(feature = "designtime")]
    {
        let payload = build_designtime_inspect_tree_payload(
            engine,
            engine_container
                .userland_definition_to_instance_traverser
                .as_ref(),
            max_depth,
        );
        return InspectTreePayload {
            status: payload.status,
            node_count: payload.node_count,
            tree_json: payload.tree_json,
            error: payload.error,
        };
    }

    #[cfg(not(feature = "designtime"))]
    {
        let _ = (engine_container, engine, max_depth);
        inspect_tree_error("inspect tree requires a designtime-enabled build")
    }
}

#[cfg(not(feature = "designtime"))]
fn inspect_tree_error(error: impl Into<String>) -> InspectTreePayload {
    InspectTreePayload {
        status: "error".to_string(),
        node_count: None,
        tree_json: None,
        error: Some(error.into()),
    }
}

fn inspect_node_list_error(error: impl Into<String>) -> InspectNodeListPayload {
    InspectNodeListPayload {
        status: "error".to_string(),
        node_count: None,
        nodes_json: None,
        error: Some(error.into()),
    }
}

fn request_slice<'a>(request_buffer: *const InterruptBuffer) -> &'a [u8] {
    let length: u64 = unsafe { (*request_buffer).length.try_into().unwrap_or_default() };
    unsafe {
        if (*request_buffer).data_ptr.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts((*request_buffer).data_ptr as *const u8, length as usize)
        }
    }
}

fn replace_node_payload_bytes(
    #[allow(unused_variables)] engine_container: &PaxEngineContainer,
    #[allow(unused_variables)] request_buffer: *const InterruptBuffer,
) -> Vec<u8> {
    #[cfg(feature = "designtime")]
    {
        let slice = request_slice(request_buffer);

        let payload = match serde_json::from_slice::<ReplaceNodeRequestPayload>(slice) {
            Ok(payload) => payload,
            Err(error) => {
                return serde_json::to_vec(&serde_json::json!({
                    "status": "error",
                    "component_type_id": "",
                    "template_node_id": 0,
                    "reload_scope": "error",
                    "reloaded_template_node_id": null,
                    "source_path": null,
                    "error": format!("failed to decode replace-node payload: {error}"),
                }))
                .unwrap_or_else(|serialization_error| {
                    format!(
                        "{{\"status\":\"error\",\"component_type_id\":\"\",\"template_node_id\":0,\"reload_scope\":\"error\",\"reloaded_template_node_id\":null,\"source_path\":null,\"error\":\"failed to decode replace-node payload and failed to serialize fallback: {}\"}}",
                        serialization_error
                    )
                    .into_bytes()
                });
            }
        };

        let response_payload = apply_designtime_replace_node_subtemplate(
            engine_container
                .userland_definition_to_instance_traverser
                .as_ref(),
            &engine_container.designtime_manager,
            &payload.component_type_id,
            payload.template_node_id,
            &payload.subtemplate,
        );
        serde_json::to_vec(&response_payload).unwrap_or_else(|error| {
            format!(
                "{{\"status\":\"error\",\"component_type_id\":\"{}\",\"template_node_id\":{},\"reload_scope\":\"error\",\"reloaded_template_node_id\":null,\"source_path\":null,\"error\":\"failed to serialize replace-node payload: {}\"}}",
                payload.component_type_id,
                payload.template_node_id,
                error
            )
            .into_bytes()
        })
    }

    #[cfg(not(feature = "designtime"))]
    {
        let _ = (engine_container, request_buffer);
        serde_json::to_vec(&serde_json::json!({
            "status": "error",
            "component_type_id": "",
            "template_node_id": 0,
            "reload_scope": "error",
            "reloaded_template_node_id": null,
            "source_path": null,
            "error": "replace-node requires a designtime-enabled build",
        }))
        .unwrap()
    }
}

fn ray_cast_payload_bytes(
    #[allow(unused_variables)] engine_container: &PaxEngineContainer,
    #[allow(unused_variables)] engine: &PaxEngine,
    #[allow(unused_variables)] request_buffer: *const InterruptBuffer,
) -> Vec<u8> {
    #[cfg(feature = "designtime")]
    {
        let slice = request_slice(request_buffer);
        let payload = match serde_json::from_slice::<RayCastRequestPayload>(slice) {
            Ok(payload) => payload,
            Err(error) => {
                return serde_json::to_vec(&inspect_node_list_error(format!(
                    "failed to decode ray-cast payload: {error}"
                )))
                .unwrap();
            }
        };

        let payload = build_designtime_ray_cast_payload(
            engine,
            engine_container
                .userland_definition_to_instance_traverser
                .as_ref(),
            payload.x,
            payload.y,
            payload.hit_invisible,
        );
        serde_json::to_vec(&InspectNodeListPayload {
            status: payload.status,
            node_count: payload.node_count,
            nodes_json: payload.nodes_json,
            error: payload.error,
        })
        .unwrap_or_else(|error| {
            serde_json::to_vec(&inspect_node_list_error(format!(
                "failed to serialize ray-cast payload: {error}"
            )))
            .unwrap()
        })
    }

    #[cfg(not(feature = "designtime"))]
    {
        let _ = (engine_container, engine, request_buffer);
        serde_json::to_vec(&inspect_node_list_error(
            "ray-cast requires a designtime-enabled build",
        ))
        .unwrap()
    }
}

fn selector_query_payload_bytes(
    #[allow(unused_variables)] engine_container: &PaxEngineContainer,
    #[allow(unused_variables)] engine: &PaxEngine,
    #[allow(unused_variables)] request_buffer: *const InterruptBuffer,
) -> Vec<u8> {
    #[cfg(feature = "designtime")]
    {
        let slice = request_slice(request_buffer);
        let payload = match serde_json::from_slice::<SelectorQueryRequestPayload>(slice) {
            Ok(payload) => payload,
            Err(error) => {
                return serde_json::to_vec(&inspect_node_list_error(format!(
                    "failed to decode selector payload: {error}"
                )))
                .unwrap();
            }
        };

        let payload = build_designtime_selector_query_payload(
            engine,
            engine_container
                .userland_definition_to_instance_traverser
                .as_ref(),
            &payload.selector,
        );
        serde_json::to_vec(&InspectNodeListPayload {
            status: payload.status,
            node_count: payload.node_count,
            nodes_json: payload.nodes_json,
            error: payload.error,
        })
        .unwrap_or_else(|error| {
            serde_json::to_vec(&inspect_node_list_error(format!(
                "failed to serialize selector payload: {error}"
            )))
            .unwrap()
        })
    }

    #[cfg(not(feature = "designtime"))]
    {
        let _ = (engine_container, engine, request_buffer);
        serde_json::to_vec(&inspect_node_list_error(
            "selector requires a designtime-enabled build",
        ))
        .unwrap()
    }
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
