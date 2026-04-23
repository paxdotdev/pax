#![allow(non_snake_case)] //Non-snake-case is used here to help denote foreign structs, e.g. from Swift via C

extern crate core;

use std::cell::RefCell;
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::mem::{transmute, ManuallyDrop};
use std::pin::Pin;
use std::rc::Rc;

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use core_graphics::context::CGContext;
use flexbuffers::DeserializationError;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_gpu::render_backend::{RenderBackend, RenderConfig};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_gpu::{Transform2D, WgpuRenderer};
use pax_runtime::api::math::Point2;
use pax_runtime::api::{
    ButtonClick, Click, ClickOrTap, Event, Focus, ModifierKey, MouseButton, MouseEventArgs,
    RenderContext, SelectStart, TextboxChange, Touch, TouchEnd, TouchMove, TouchStart,
};
use pax_runtime::engine::layer_tiling::{scroller_canvas_plan_with_policy, ScrollerTilingPolicy};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use pax_runtime::pax_gpu_render_context::{
    LayerRenderer, LayerSurfaceEntry, LayerSurfaceLayout, LayerSurfaceSize, LayerTarget,
    PaxGpuRenderer,
};
use pax_runtime::PaxEngine;
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet::kurbo;
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet::kurbo::Shape;
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet::{InterpolationMode, LineCap, RenderContext as PietRenderContext, StrokeStyle};
#[cfg(not(any(target_os = "ios", target_os = "macos")))]
use piet_coregraphics::CoreGraphicsContext;
#[cfg(feature = "designtime")]
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
    fn fill_with_opacity(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        brush: &pax_runtime::api::Fill,
        opacity: f64,
    ) {
        self.backend.fill(
            path.clone(),
            &fill_to_piet_brush(&brush.with_alpha_factor(opacity), path.bounding_box()),
        );
    }

    fn stroke_with_opacity(
        &mut self,
        _layer: usize,
        path: kurbo::BezPath,
        stroke: &pax_runtime::api::Stroke,
        opacity: f64,
    ) {
        let width = stroke.width.get().expect_pixels().to_float();
        let brush = fill_to_piet_brush(
            &pax_runtime::api::Fill::Solid(stroke.color.get()).with_alpha_factor(opacity),
            path.bounding_box(),
        );
        self.backend.stroke_styled(
            path.clone(),
            &brush,
            width,
            &stroke_to_piet_style(stroke.cap.get()),
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

    fn refresh_layers(&mut self, _layers: &[usize]) {}
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

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
fn stroke_to_piet_style(cap: pax_runtime::api::StrokeCap) -> StrokeStyle {
    let mut style = StrokeStyle::new();
    style.set_line_cap(match cap {
        pax_runtime::api::StrokeCap::Butt => LineCap::Butt,
        pax_runtime::api::StrokeCap::Round => LineCap::Round,
        pax_runtime::api::StrokeCap::Square => LineCap::Square,
    });
    style
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Clone)]
struct SurfaceRegistration {
    key: String,
    host_signature: String,
    origin_x: f32,
    origin_y: f32,
    replay_priority: i32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
    layer_ptr: *mut c_void,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Default)]
struct LayerSurfaceLayoutData {
    active: bool,
    surfaces: Vec<SurfaceRegistration>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[derive(Default)]
struct LayerSurfaceRegistry {
    layers: Vec<LayerSurfaceLayoutData>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl LayerSurfaceRegistry {
    fn begin_frame(&mut self, layer_count: usize) {
        if self.layers.len() < layer_count {
            self.layers
                .resize_with(layer_count, LayerSurfaceLayoutData::default);
        }
        for layer in &mut self.layers {
            layer.active = false;
            layer.surfaces.clear();
        }
    }

    fn set_layer_active(&mut self, layer_id: usize, active: bool) {
        if self.layers.len() <= layer_id {
            self.layers
                .resize_with(layer_id + 1, LayerSurfaceLayoutData::default);
        }
        if let Some(layer) = self.layers.get_mut(layer_id) {
            layer.active = active;
        }
    }

    fn register_surface(&mut self, layer_id: usize, surface: SurfaceRegistration) {
        if self.layers.len() <= layer_id {
            self.layers
                .resize_with(layer_id + 1, LayerSurfaceLayoutData::default);
        }
        if let Some(layer) = self.layers.get_mut(layer_id) {
            layer.surfaces.push(surface);
        }
    }

    fn layout_for_layer(&self, layer_id: usize) -> LayerSurfaceLayout {
        let Some(layer) = self.layers.get(layer_id) else {
            return LayerSurfaceLayout {
                surfaces: Vec::new(),
                active: false,
            };
        };
        let surfaces = layer
            .surfaces
            .iter()
            .map(|surface| LayerSurfaceEntry {
                key: surface.key.clone(),
                host_signature: surface.host_signature.clone(),
                origin_x: surface.origin_x,
                origin_y: surface.origin_y,
                replay_priority: surface.replay_priority,
                surface: LayerSurfaceSize {
                    logical_width: surface.logical_width,
                    logical_height: surface.logical_height,
                    surface_width: surface.surface_width,
                    surface_height: surface.surface_height,
                    dpr: surface.dpr,
                },
            })
            .collect();
        LayerSurfaceLayout {
            surfaces,
            active: layer.active,
        }
    }

    fn registrations_for_layer(&self, layer_id: usize) -> Vec<SurfaceRegistration> {
        self.layers
            .get(layer_id)
            .map(|layer| layer.surfaces.clone())
            .unwrap_or_default()
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub struct AppleRenderContext {
    renderer: PaxGpuRenderer,
    registry: Rc<RefCell<LayerSurfaceRegistry>>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl AppleRenderContext {
    fn new() -> Self {
        let registry = Rc::new(RefCell::new(LayerSurfaceRegistry::default()));
        let registry_factory = Rc::clone(&registry);
        let renderer = PaxGpuRenderer::new(move |layer| {
            let registry = Rc::clone(&registry_factory);
            Box::pin(async move {
                let initial_layout = registry.borrow().layout_for_layer(layer);
                let registrations = registry.borrow().registrations_for_layer(layer);
                if registrations.is_empty() {
                    let layout_provider: Pin<Box<dyn Fn() -> LayerSurfaceLayout>> = Box::pin({
                        let registry = Rc::clone(&registry);
                        move || registry.borrow().layout_for_layer(layer)
                    });
                    let target = LayerTarget::new(Vec::new(), initial_layout.active);
                    return Some((target, layout_provider));
                }

                let mut renderers = Vec::with_capacity(registrations.len());
                for surface in registrations {
                    if surface.layer_ptr.is_null() {
                        log::warn!(
                            "skipping render backend for layer {} tile {}: nil layer pointer",
                            layer,
                            surface.key
                        );
                        return None;
                    }
                    let backend = match unsafe {
                        RenderBackend::to_core_animation_layer(
                            surface.layer_ptr,
                            RenderConfig::new(
                                false,
                                surface.surface_width,
                                surface.surface_height,
                                surface.dpr,
                            ),
                        )
                    }
                    .await
                    {
                        Ok(backend) => backend,
                        Err(err) => {
                            log::warn!(
                                "failed to create render backend for layer {} tile {}: {}",
                                layer,
                                surface.key,
                                err
                            );
                            return None;
                        }
                    };

                    let mut renderer = WgpuRenderer::new(backend);
                    renderer.set_surface_transform(Transform2D::from_array([
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        -surface.origin_x,
                        -surface.origin_y,
                    ]));
                    renderer.resize_surface(
                        surface.surface_width as f32,
                        surface.surface_height as f32,
                    );
                    renderer.set_viewport(
                        surface.logical_width,
                        surface.logical_height,
                        surface.dpr,
                    );
                    renderers.push(LayerRenderer::new(
                        surface.key,
                        surface.host_signature,
                        renderer,
                        surface.origin_x,
                        surface.origin_y,
                        surface.logical_width,
                        surface.logical_height,
                        surface.surface_width,
                        surface.surface_height,
                        surface.dpr,
                    ));
                }

                let layout_provider: Pin<Box<dyn Fn() -> LayerSurfaceLayout>> = Box::pin({
                    let registry = Rc::clone(&registry);
                    move || registry.borrow().layout_for_layer(layer)
                });
                let layout = layout_provider();
                let mut target = LayerTarget::new(renderers, layout.active);
                for (surface, renderer) in layout
                    .surfaces
                    .iter()
                    .zip(target.renderers_mut().iter_mut())
                {
                    renderer.renderer_mut().resize_surface(
                        surface.surface.surface_width as f32,
                        surface.surface.surface_height as f32,
                    );
                    renderer.renderer_mut().set_viewport(
                        surface.surface.logical_width,
                        surface.surface.logical_height,
                        surface.surface.dpr,
                    );
                }
                Some((target, layout_provider))
            })
        });
        Self { renderer, registry }
    }

    fn registry(&self) -> Rc<RefCell<LayerSurfaceRegistry>> {
        Rc::clone(&self.registry)
    }

    fn renderer_mut(&mut self) -> &mut PaxGpuRenderer {
        &mut self.renderer
    }
}

#[cfg(target_os = "ios")]
pub fn native_scroller_tiling_policy() -> ScrollerTilingPolicy {
    let mut policy = ScrollerTilingPolicy::default();
    // iOS has not shown WebGL-style uninit pressure on native Metal surfaces in the stress bed.
    // Prefer fewer/larger surfaces and a tight warm band to reduce replay work on constrained
    // mobile GPUs.
    policy.target_tile_backing_dimension = 4096.0;
    policy.prewarm_viewport_pad_x_multiplier = 0.5;
    policy.prewarm_viewport_pad_y_multiplier = 0.5;
    policy.prewarm_viewport_pad_min_x = 384.0;
    policy.prewarm_viewport_pad_min_y = 384.0;
    policy.max_surfaces_per_layer = Some(12);
    policy
}

#[cfg(target_os = "macos")]
pub fn native_scroller_tiling_policy() -> ScrollerTilingPolicy {
    let mut policy = ScrollerTilingPolicy::default();
    // Large native macOS windows at 2x DPR can become two-dimensional tile grids if the backing
    // target is too small. That increases replay pressure and makes freshly-retargeted tiles
    // visible before replay catches up, even on slow scrolls. Keep tiles large enough to cover
    // common full-width windows with a single column.
    policy.target_tile_backing_dimension = 4096.0;
    policy.prewarm_viewport_pad_x_multiplier = 0.5;
    policy.prewarm_viewport_pad_y_multiplier = 0.5;
    policy.prewarm_viewport_pad_min_x = 384.0;
    policy.prewarm_viewport_pad_min_y = 384.0;
    policy.max_surfaces_per_layer = Some(12);
    policy
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

fn serialize_payload<T: Serialize>(payload: &T) -> *mut NativeMessageQueue {
    let mut serializer = flexbuffers::FlexbufferSerializer::new();
    payload.serialize(&mut serializer).unwrap();
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

#[cfg(any(target_os = "ios", target_os = "macos"))]
fn ensure_render_context(container: &mut PaxEngineContainer) -> &mut AppleRenderContext {
    if container._render_context.is_null() {
        container._render_context = Box::into_raw(Box::new(AppleRenderContext::new()));
    }
    unsafe { &mut *container._render_context }
}

#[derive(Serialize)]
struct InspectTreePayload {
    status: String,
    node_count: Option<usize>,
    tree_json: Option<String>,
    error: Option<String>,
}

#[cfg(feature = "designtime")]
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

#[cfg(feature = "designtime")]
#[derive(Deserialize)]
struct SelectorQueryRequestPayload {
    selector: String,
}

#[cfg(feature = "designtime")]
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
            let modifiers = args.modifiers.iter().map(ModifierKey::from).collect();
            let args_click = Click {
                mouse: MouseEventArgs {
                    x: args.x,
                    y: args.y,
                    button: MouseButton::from(args.button.clone()),
                    modifiers,
                },
            };
            if let Some(topmost_node) = engine
                .runtime_context
                .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
            {
                topmost_node.dispatch_click(
                    Event::new(args_click),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::ClickOrTap(args) => {
            if let Some(topmost_node) = engine
                .runtime_context
                .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
            {
                topmost_node.dispatch_click_or_tap(
                    Event::new(ClickOrTap {
                        x: args.x,
                        y: args.y,
                    }),
                    &globals,
                    &engine.runtime_context,
                );
            }
        }
        NativeInterrupt::TouchStart(args) => {
            if let Some(first_touch) = args.touches.first() {
                let touches = args.touches.iter().map(Touch::from).collect();
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                {
                    topmost_node.dispatch_touch_start(
                        Event::new(TouchStart { touches }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
        }
        NativeInterrupt::TouchMove(args) => {
            if let Some(first_touch) = args.touches.first() {
                let touches = args.touches.iter().map(Touch::from).collect();
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                {
                    topmost_node.dispatch_touch_move(
                        Event::new(TouchMove { touches }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
        }
        NativeInterrupt::TouchEnd(args) => {
            if let Some(first_touch) = args.touches.first() {
                let touches = args.touches.iter().map(Touch::from).collect();
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                {
                    topmost_node.dispatch_touch_end(
                        Event::new(TouchEnd { touches }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
        }
        NativeInterrupt::FormRadioListChange(args) => {
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
        NativeInterrupt::ScrollerPosition(args) => {
            let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
            if let Some(node) = node {
                borrow!(node.instance_node).handle_native_interrupt(&node, &interrupt);
            }
        }
        NativeInterrupt::Scroll(_args) => {}
        NativeInterrupt::VisualViewportUpdate(_args) => {}
        NativeInterrupt::AddedLayer(args) => {
            if let Some(layer_id) = args.layer_id.map(|layer| layer as usize) {
                engine.runtime_context.set_canvas_dirty(layer_id);
                engine
                    .runtime_context
                    .mark_canvas_nodes_on_layer_dirty(layer_id);
            } else {
                engine.runtime_context.set_all_canvases_dirty();
                engine.runtime_context.mark_all_canvas_nodes_dirty();
            }
        }
        NativeInterrupt::Image(args) => match args {
            ImageLoadInterruptArgs::Reference(_ref_args) => {
                #[cfg(any(target_os = "ios", target_os = "macos"))]
                {
                    let ref_args = _ref_args;
                    let render_context = ensure_render_context(&mut engine_container);
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
                    render_context.renderer_mut().load_image(
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

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);
}

/// Perform full tick of engine, including property computation, lifecycle event handling, and rendering side-effects.
/// Returns a message queue of native rendering actions encoded as a Flexbuffer via FFI to Swift.
/// The returned message queue requires explicit deallocation: `pax_deallocate_message_queue`
#[no_mangle] //Exposed to Swift via PaxCartridge.h
pub extern "C" fn pax_tick(
    engine_container: *mut PaxEngineContainer,
    _render_target: *mut c_void,
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

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let will_cast_cgContext = _render_target as *mut CGContext;
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
pub extern "C" fn pax_get_layer_canvas_plan(
    engine_container: *mut PaxEngineContainer,
    layer_id: u32,
    dpr: f32,
) -> *mut NativeMessageQueue {
    if engine_container.is_null() {
        return std::ptr::null_mut();
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let engine = unsafe { Box::from_raw(engine_container._engine) };
    let ctx = &engine.runtime_context;
    let dpr = (dpr as f64).max(1.0);
    let layer = layer_id as usize;

    let plan = if let Some(owner) = ctx.get_layer_scroller_owner(layer) {
        let scroller_id = owner.to_u32();
        if let Some(state) = ctx.get_scroller_surface_state(scroller_id) {
            let host_signature = format!("scroller:{scroller_id}");
            let mut viewport_width = state.viewport_width;
            let mut viewport_height = state.viewport_height;
            let mut scroll_x = if state.presentation_scroll_x.is_finite() {
                state.presentation_scroll_x
            } else {
                state.scroll_x
            };
            let mut scroll_y = if state.presentation_scroll_y.is_finite() {
                state.presentation_scroll_y
            } else {
                state.scroll_y
            };
            if ctx.get_root_scroller_id() == Some(scroller_id) {
                if let Some(visual) = ctx.get_visual_viewport_state() {
                    viewport_width = visual.width;
                    viewport_height = visual.height;
                    scroll_x = visual.page_scroll_x;
                    scroll_y = visual.page_scroll_y;
                }
            }
            Some(scroller_canvas_plan_with_policy(
                layer,
                host_signature,
                state.content_width,
                state.content_height,
                viewport_width,
                viewport_height,
                scroll_x,
                scroll_y,
                dpr,
                engine.scroller_tiling_policy,
            ))
        } else {
            None
        }
    } else if layer == 0 {
        let viewport = ctx.globals().viewport.get();
        let host_signature = "root".to_string();
        let width = viewport.bounds.0;
        let height = viewport.bounds.1;
        Some(scroller_canvas_plan_with_policy(
            layer,
            host_signature,
            width,
            height,
            width,
            height,
            0.0,
            0.0,
            dpr,
            engine.scroller_tiling_policy,
        ))
    } else {
        None
    };

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);

    match plan {
        Some(plan) => serialize_payload(&plan),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn pax_surface_registry_begin_frame(
    engine_container: *mut PaxEngineContainer,
    layer_count: u32,
) {
    if engine_container.is_null() {
        return;
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let render_context = ensure_render_context(&mut engine_container);
        let registry = render_context.registry();
        registry.borrow_mut().begin_frame(layer_count as usize);
    }
    let _ = Box::into_raw(engine_container);
}

#[no_mangle]
pub extern "C" fn pax_surface_registry_set_layer_active(
    engine_container: *mut PaxEngineContainer,
    layer_id: u32,
    active: bool,
) {
    if engine_container.is_null() {
        return;
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let render_context = ensure_render_context(&mut engine_container);
        let registry = render_context.registry();
        registry
            .borrow_mut()
            .set_layer_active(layer_id as usize, active);
    }
    let _ = Box::into_raw(engine_container);
}

#[no_mangle]
pub extern "C" fn pax_surface_registry_register_surface(
    engine_container: *mut PaxEngineContainer,
    layer_id: u32,
    key_ptr: *const std::os::raw::c_char,
    host_signature_ptr: *const std::os::raw::c_char,
    origin_x: f32,
    origin_y: f32,
    replay_priority: i32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr_x: f32,
    dpr_y: f32,
    layer_ptr: *mut c_void,
) {
    if engine_container.is_null() || key_ptr.is_null() || host_signature_ptr.is_null() {
        return;
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let key = unsafe { CStr::from_ptr(key_ptr) }
            .to_string_lossy()
            .to_string();
        let host_signature = unsafe { CStr::from_ptr(host_signature_ptr) }
            .to_string_lossy()
            .to_string();
        let render_context = ensure_render_context(&mut engine_container);
        let registry = render_context.registry();
        registry.borrow_mut().register_surface(
            layer_id as usize,
            SurfaceRegistration {
                key,
                host_signature,
                origin_x,
                origin_y,
                replay_priority,
                logical_width,
                logical_height,
                surface_width,
                surface_height,
                dpr: [dpr_x, dpr_y],
                layer_ptr,
            },
        );
    }
    let _ = Box::into_raw(engine_container);
}

#[no_mangle]
pub extern "C" fn pax_refresh_render_surfaces(engine_container: *mut PaxEngineContainer) {
    if engine_container.is_null() {
        return;
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let render_context = ensure_render_context(&mut engine_container);
        let layer_count = render_context.renderer_mut().layers();
        let layers: Vec<usize> = (0..layer_count).collect();
        render_context.renderer_mut().refresh_layers(&layers);
    }
    let _ = Box::into_raw(engine_container);
}

#[no_mangle]
pub extern "C" fn pax_render(engine_container: *mut PaxEngineContainer) {
    if engine_container.is_null() {
        return;
    }
    let mut engine_container = unsafe { Box::from_raw(engine_container) };
    let mut engine = unsafe { Box::from_raw(engine_container._engine) };

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        let render_context = ensure_render_context(&mut engine_container);
        let renderer = render_context.renderer_mut();
        for layer_id in renderer.take_ready_canvas_layers() {
            engine.runtime_context.set_canvas_dirty(layer_id);
            engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer_id);
        }
        for layer_id in renderer.take_replay_canvas_layers() {
            engine.runtime_context.set_canvas_dirty(layer_id);
            engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer_id);
        }
        engine.render(renderer as &mut dyn RenderContext);
    }

    engine_container._engine = Box::into_raw(engine);
    let _ = Box::into_raw(engine_container);
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

#[cfg(feature = "designtime")]
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
