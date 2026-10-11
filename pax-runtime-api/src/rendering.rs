//! Rendering backend contracts and helpers for drawing Pax scene content.

use super::*;

/// One vector paint contributing alpha to a mask, in local path coordinates.
/// Color channels do not affect coverage; fill alpha and opacity are multiplied.
#[derive(Clone, Debug)]
pub struct AlphaMaskPaint {
    pub path: kurbo::BezPath,
    pub transform: kurbo::Affine,
    pub fill: Paint,
    pub opacity: f64,
    /// Complete local geometry bounds before stroke expansion or trimming.
    pub paint_bounds: kurbo::Rect,
    /// Primitive boundary: compose its paints before applying source-relative opacity.
    pub composition: Option<(u32, f32)>,
}

/// Replay invalidation for one logical canvas layer.
#[derive(Clone, Debug)]
pub struct ReplayCanvasLayerUpdate {
    pub layer: usize,
    /// Canvas-content regions requiring replay. The runtime selects nodes from its
    /// shared scene geometry; `None` requests full-layer dirtification.
    pub regions: Option<Vec<kurbo::Rect>>,
}

/// One authored opacity boundary, ordered from the outermost ancestor to the painted node.
/// Native surfaces can present independently while a canvas backend composes each scope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpacityScope {
    pub node_id: u32,
    pub opacity: f32,
}

impl crate::Interpolatable for OpacityScope {}

/// The drawing contract shared by Pax runtime backends.
pub trait RenderContext {
    /// Whether this session uses Pax's GPU renderer. This identifies the backend,
    /// not individual effect support or the presence of hardware acceleration.
    fn is_gpu(&self) -> bool {
        false
    }

    /// Begin an experimental GPU-only subtree effect. Returns whether source draws need replay.
    /// GPU adapters must implement this hook. The runtime bypasses it on CPU,
    /// preserving ordinary children and reporting a bounded diagnostic.
    /// Available with the opt-in `shaders` Cargo feature.
    #[cfg(feature = "shaders")]
    fn begin_shader_effect(
        &mut self,
        _layer: usize,
        _id: u32,
        _z: i32,
        _effect: &ShaderEffect,
        _transform: kurbo::Affine,
        _bounds: (f64, f64),
        _scopes: &[OpacityScope],
        _children: &[u32],
    ) -> bool {
        panic!("shader effects require the WGPU renderer; this backend does not support them")
    }
    /// Finish the boundary begun by `begin_shader_effect`.
    #[cfg(feature = "shaders")]
    fn end_shader_effect(&mut self, _layer: usize, _id: u32) {}
    /// Optional diagnostics for an Image whose decoded pixels are not available yet.
    /// This does not change readiness, scheduling, or component lifecycle.
    fn image_pending(&mut self, _layer: usize, _node_id: u32, _path: &str) {}

    // Drawing
    /// Fills a path at full opacity.
    fn fill(&mut self, layer: usize, path: kurbo::BezPath, fill: &Paint) {
        self.fill_with_opacity(layer, path, fill, 1.0);
    }
    /// Fills a path with an explicit opacity multiplier.
    fn fill_with_opacity(&mut self, layer: usize, path: kurbo::BezPath, fill: &Paint, opacity: f64);
    /// Fills a path with a material and explicit opacity multiplier.
    fn fill_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &Paint,
        _material: &ResolvedMaterial,
        opacity: f64,
    ) {
        self.fill_with_opacity(layer, path, fill, opacity);
    }
    /// Fills a path after optional geometry smoothing.
    fn fill_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &Paint,
        material: &ResolvedMaterial,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        let path = crate::drawing::path_smoothing::smooth_bez_path(&path, smoothing);
        self.fill_with_material_and_opacity(layer, path, fill, material, opacity);
    }
    /// Strokes a path at full opacity.
    fn stroke(&mut self, layer: usize, path: kurbo::BezPath, stroke: &ResolvedStroke) {
        self.stroke_with_opacity(layer, path, stroke, 1.0);
    }
    /// Strokes a path with an explicit opacity multiplier.
    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &ResolvedStroke,
        opacity: f64,
    );
    /// Strokes a path with a material and explicit opacity multiplier.
    fn stroke_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &ResolvedStroke,
        _material: &ResolvedMaterial,
        opacity: f64,
    ) {
        self.stroke_with_opacity(layer, path, stroke, opacity);
    }
    /// Strokes a path after optional geometry smoothing.
    fn stroke_with_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        smoothing: PathSmoothing,
    ) {
        let path = crate::drawing::path_smoothing::smooth_bez_path(&path, smoothing);
        self.stroke_with_material_and_opacity(layer, path, stroke, material, opacity);
    }
    /// Strokes a path with a normalized visible range over the path's total length.
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
    );
    /// Strokes a draw-ranged path after optional geometry smoothing.
    fn stroke_with_draw_range_and_material_and_opacity_and_smoothing(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &ResolvedStroke,
        material: &ResolvedMaterial,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
        smoothing: PathSmoothing,
    ) {
        let path = crate::drawing::path_smoothing::smooth_bez_path(&path, smoothing);
        self.stroke_with_draw_range_and_material_and_opacity(
            layer, path, stroke, material, opacity, draw_start, draw_end,
        );
    }

    // Clip/transform
    /// Saves the current graphics state for a layer.
    fn save(&mut self, layer: usize);
    /// Restores the previous graphics state for a layer.
    fn restore(&mut self, layer: usize);
    /// Applies a clipping path to a layer.
    fn clip(&mut self, layer: usize, path: BezPath);
    /// Clips canvas drawing by source-over vector paint alpha, optionally
    /// feathered with a Gaussian of this standard deviation in logical pixels.
    fn clip_alpha(&mut self, _layer: usize, _paints: &[AlphaMaskPaint], _feather: f64) {
        panic!("this rendering backend does not implement alpha masks");
    }
    /// Whether alpha sources can use ordinary retained vector rendering.
    fn supports_alpha_source_capture(&self) -> bool {
        false
    }
    /// Destination layer while recording a detached alpha source. Source coordinates are world
    /// coordinates; `begin_alpha_source` supplies the mapping to the destination surface.
    fn alpha_source_layer(&self) -> Option<usize> {
        None
    }
    /// Isolates source draws from visible content and inherited clips. The currently open node
    /// owns the result. Scopes before `inherited_scopes` belong to the consumer, not the source.
    fn begin_alpha_source(
        &mut self,
        _layer: usize,
        _world_to_surface: kurbo::Affine,
        _inherited_scopes: usize,
        _feather: f64,
    ) {
        panic!("this rendering backend does not implement alpha-source capture");
    }
    /// Completes the source and installs its captured alpha in the consumer's clip stack.
    fn end_alpha_source(&mut self, _layer: usize) {}
    /// Applies an affine transform to a layer.
    fn transform(&mut self, layer: usize, affine: kurbo::Affine);

    // Images
    /// Loads raw RGBA image bytes under an identifier.
    fn load_image(&mut self, identifier: &str, image: &[u8], width: usize, height: usize);
    /// Draws a previously loaded image into `rect` at full opacity.
    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect) {
        self.draw_image_with_opacity(layer, image_path, rect, 1.0);
    }
    /// Draws an image with an opacity multiplier, in addition to source pixel alpha.
    /// Backends must apply this at draw time without changing the cached source image.
    fn draw_image_with_opacity(
        &mut self,
        layer: usize,
        image_path: &str,
        rect: kurbo::Rect,
        opacity: f64,
    );
    /// Returns the loaded image size, if known.
    fn get_image_size(&mut self, image_path: &str) -> Option<(usize, usize)>;
    /// Returns true when the image is available for drawing.
    fn image_loaded(&self, image_path: &str) -> bool;

    // Other
    /// Returns the current number of render layers.
    fn layers(&self) -> usize;
    /// Resizes the backend layer set to `layer_count`.
    fn resize_layers_to(&mut self, layer_count: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>);
    /// Clears a layer.
    fn clear(&mut self, layer: usize);
    /// Flushes a layer to its presentation target.
    fn flush(&mut self, layer: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>);
    /// Resizes the render surface.
    fn resize(&mut self, width: usize, height: usize);
    /// Requests refresh of specific layers.
    fn refresh_layers(&mut self, layers: &[usize]);
    /// Installs resolved lighting for a logical canvas layer.
    fn set_scene_lighting(&mut self, _layer: usize, _lighting: &SceneLighting) {}
    /// Cancels any narrowed replay scope for a layer so subsequent work targets all surfaces.
    fn clear_targeted_replay(&mut self, _layer: usize) {}
    /// Collects completed canvas layers without starting new initialization work.
    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        vec![]
    }

    /// Returns canvas layers that should replay prior draw commands.
    fn take_replay_canvas_layers(&mut self) -> Vec<usize> {
        vec![]
    }

    /// Returns canvas layer replay work, optionally narrowed to canvas-content regions.
    fn take_replay_canvas_layer_updates(&mut self) -> Vec<ReplayCanvasLayerUpdate> {
        self.take_replay_canvas_layers()
            .into_iter()
            .map(|layer| ReplayCanvasLayerUpdate {
                layer,
                regions: None,
            })
            .collect()
    }

    /// Requests a screenshot for one render layer.
    fn request_layer_screenshot(&mut self, _layer: usize, _request_id: u32) {}

    /// Returns a previously requested layer screenshot, if ready.
    fn take_layer_screenshot(&mut self, _layer: usize, _request_id: u32) -> Option<ScreenshotData> {
        None
    }

    /// Returns ready screenshots for the physical surfaces backing one logical layer.
    fn take_layer_surface_screenshots(
        &mut self,
        layer: usize,
        request_id: u32,
    ) -> Vec<LayerSurfaceScreenshotData> {
        self.take_layer_screenshot(layer, request_id)
            .map(|screenshot| {
                vec![LayerSurfaceScreenshotData {
                    id: screenshot.id,
                    key: "single".to_string(),
                    data: screenshot.data,
                    width: screenshot.width,
                    height: screenshot.height,
                    origin_x: 0.0,
                    origin_y: 0.0,
                    logical_width: screenshot.width as f32,
                    logical_height: screenshot.height as f32,
                }]
            })
            .unwrap_or_default()
    }

    /// Returns whether the backend retains individual canvas nodes across frames.
    fn retains_canvas_nodes(&self) -> bool {
        true
    }

    /// Whether surface replay is independent of ordinary content recording. Such backends
    /// update all affected resident surfaces for content changes even while replay is pending.
    fn separates_surface_replay(&self) -> bool {
        false
    }

    /// Queue valid logical drawings for surface replay and return only cache misses that
    /// need component rendering. Called after scene changes settle, before content recording.
    /// The default immediate/legacy path needs all selected nodes to render again.
    fn prepare_canvas_replay(&mut self, _layer: usize, nodes: &[u32]) -> Vec<u32> {
        nodes.to_vec()
    }

    /// Opt-in presentation-only jobs, ordered by current visibility/scroll priority.
    /// Returned regions must cover all selected physical surfaces. Chassis may call this
    /// only against a settled scene; unsupported backends leave their queues untouched.
    fn take_settled_surface_replays(&mut self) -> Vec<ReplayCanvasLayerUpdate> {
        Vec::new()
    }

    /// Current rank of a selected replay job. Nonpositive ranks are visible;
    /// positive ranks are speculative. Re-read after presentation/plan changes.
    fn surface_replay_priority(&self, _layer: usize) -> i32 {
        0
    }

    /// Opt in to visible-first ordinary drawing for this logical layer. The rank
    /// describes all currently installed surfaces, not only a selected replay job:
    /// nonpositive means visible, positive means warm/offscreen. Layers are drawn
    /// atomically with their ancestor hooks before becoming eligible for replay.
    /// `None` preserves the ordinary unsplit path for unsupported backends.
    fn ordinary_layer_priority(&self, _layer: usize) -> Option<i32> {
        None
    }

    /// Starts at most one initialization unit ranked strictly before `before`.
    /// Visible ranks compare as zero, so an already-runnable visible replay wins
    /// over more creation. Returns whether work started, not whether it completed.
    /// Async completions notify the host; they must not recursively dispatch work
    /// outside the preparation allowance. Unsupported backends return false.
    fn start_surface_initialization(&mut self, _before: i32) -> bool {
        false
    }

    /// Notify the chassis after asynchronous surface completion. The callback must
    /// schedule work, never synchronously reenter the renderer. Clear it on teardown.
    fn set_surface_ready_callback(&mut self, _callback: Option<Rc<dyn Fn()>>) {}

    /// Requeue an unsubmitted job without discarding its physical replay scope.
    fn defer_surface_replay(&mut self, _layer: usize) {}

    /// Queue all requested cached drawings atomically. A cache miss must leave the
    /// target unchanged and return false, so a later capture or ordinary render can repair it.
    fn prepare_settled_surface_replay(&mut self, _layer: usize, _nodes: &[u32]) -> bool {
        false
    }

    /// Whether the chassis should leave surface jobs with the bounded preparation
    /// pass instead of converting them into ordinary scene dirtiness.
    fn prepares_surface_content(&self) -> bool {
        false
    }

    /// Missing immutable drawings for a current surface job. `None` means this
    /// backend does not support recording cold content separately from submission.
    fn missing_surface_drawings(&mut self, _layer: usize, _nodes: &[u32]) -> Option<Vec<u32>> {
        None
    }

    /// Begin an isolated, CPU-only drawing capture for one layer. Drawing hooks
    /// may run, but no GPU target may change or become presentable during capture.
    fn begin_surface_recording(&mut self, _layer: usize) -> bool {
        false
    }

    /// Finish capture and discard transient graphics state. Completed immutable
    /// records may survive; incomplete records must not become replayable. Return
    /// false if capacity pressure requires the atomic direct-drawing fallback.
    fn end_surface_recording(&mut self, _layer: usize) -> bool {
        false
    }

    /// Begin an atomic direct-drawing fallback for the selected surface job when
    /// its drawings cannot fit in the retained command cache. Until `flush`, all
    /// drawing must be restricted to that job's targets. Defaults to unsupported.
    fn begin_surface_fallback(&mut self, _layer: usize) -> bool {
        false
    }

    /// Begins rendering a node and returns false when the backend can skip it.
    fn begin_node(
        &mut self,
        _layer: usize,
        _node_id: u32,
        _z_index: i32,
        _light_mask: u32,
    ) -> bool {
        true
    }

    /// Begins rendering a node with conservative canvas-space coverage bounds.
    fn begin_node_with_bounds(
        &mut self,
        layer: usize,
        node_id: u32,
        z_index: i32,
        _coverage_bounds: kurbo::Rect,
        light_mask: u32,
    ) -> bool {
        self.begin_node(layer, node_id, z_index, light_mask)
    }

    /// Returns true when a bounded node skipped by `begin_node_with_bounds` is clean for now.
    fn take_clean_skipped_node(&mut self, _layer: usize, _node_id: u32) -> bool {
        false
    }

    /// Whether canvas opacity is applied after composing each subtree's canvas
    /// content. This does not imply that the backend caches content across frames.
    fn supports_subtree_opacity(&self) -> bool {
        false
    }

    /// Assigns stable opacity ancestry to the currently recorded canvas node.
    fn set_node_opacity_scopes(&mut self, _layer: usize, _node_id: u32, _scopes: &[OpacityScope]) {}

    /// Ends rendering a node and returns true when the node was recorded.
    fn end_node(&mut self, _layer: usize, _node_id: u32) -> bool {
        true
    }

    /// Removes a node from backend retained state, if any.
    fn remove_node(&mut self, _layer: usize, _node_id: u32) -> bool {
        true
    }
}

/// Convert a kurbo BezPath to a SVG-friendly drawing string
pub fn bez_path_to_svg_path_data(path: &BezPath) -> String {
    use kurbo::PathEl;

    let mut data = String::new();
    for element in path.elements() {
        match element {
            PathEl::MoveTo(point) => data.push_str(&format!("M{:.3},{:.3}", point.x, point.y)),
            PathEl::LineTo(point) => data.push_str(&format!("L{:.3},{:.3}", point.x, point.y)),
            PathEl::QuadTo(ctrl, point) => data.push_str(&format!(
                "Q{:.3},{:.3} {:.3},{:.3}",
                ctrl.x, ctrl.y, point.x, point.y
            )),
            PathEl::CurveTo(ctrl1, ctrl2, point) => data.push_str(&format!(
                "C{:.3},{:.3} {:.3},{:.3} {:.3},{:.3}",
                ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, point.x, point.y
            )),
            PathEl::ClosePath => data.push('Z'),
        }
    }
    data
}

/// Render layer selected for a primitive or native surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Layer {
    /// Platform-native element layer.
    Native,
    /// Platform-native layer that does not participate in occlusion.
    NativeNonOccluding,
    /// GPU/canvas-rendered layer.
    Canvas,
    /// Runtime can choose the appropriate layer.
    DontCare,
}
