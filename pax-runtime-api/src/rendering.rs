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

/// Unprojected lighting frame of a planar draw, in logical world pixels.
/// Geometry stays local: `origin + x * tangent_x + y * tangent_y` supplies its
/// shading position independently of canvas tiles, captures, or projection.
/// The oriented normal is unit length and perpendicular to both tangents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightingSurface {
    pub origin: [f32; 3],
    pub tangent_x: [f32; 3],
    pub tangent_y: [f32; 3],
    pub normal: [f32; 3],
}

/// Depth of a flat surface as `z = x*a + y*b + c` in canvas-content coordinates.
/// Positive depth is toward the default orthographic viewer. Coplanar surfaces
/// retain paint order; a spatial back face reverses its planar layer stack.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DepthPlane {
    /// Independent presentation domain. Higher domains paint above lower ones
    /// regardless of geometric depth; 0 is the scene and positive values rank
    /// viewport Overlay contributions and temporary transfer presentations.
    pub presentation_layer: u32,
    pub coefficients: [f64; 3],
    pub backface: bool,
}

impl DepthPlane {
    /// Equality line in a common projected chart, before any pixel-distance
    /// normalization. Its positive side is where `self` is nearer than `other`.
    pub fn equality_line(self, other: Self) -> [f64; 3] {
        std::array::from_fn(|i| self.coefficients[i] - other.coefficients[i])
    }

    /// Tests coincidence over a convex region's vertices, in shared depth units.
    /// This tolerance is independent of a consumer's pixel-distance split tolerance.
    pub fn coincides_over(self, other: Self, vertices: impl IntoIterator<Item = [f64; 2]>) -> bool {
        self.presentation_layer == other.presentation_layer
            && vertices
                .into_iter()
                .all(|[x, y]| (self.depth_at(x, y) - other.depth_at(x, y)).abs() <= 1e-7)
    }
    /// Evaluates camera-relative depth at a projected coordinate.
    pub fn depth_at(self, x: f64, y: f64) -> f64 {
        self.coefficients[0] * x + self.coefficients[1] * y + self.coefficients[2]
    }

    /// Stable back-to-front key for geometrically coincident paint. Larger authored
    /// orders paint later on front faces and earlier on back faces. Opposing faces
    /// at exactly the same depth put front-facing paint last, independently of the
    /// authored index range. Renderers, occlusion masks, and picking share this tie.
    pub fn coplanar_order(self, authored_order: i64) -> (bool, i64) {
        (
            !self.backface,
            if self.backface {
                !authored_order
            } else {
                authored_order
            },
        )
    }

    /// Compares depths for point picking, from farther to nearer.
    ///
    /// The existing picking policy rounds to steps of `1e-7` in the view's depth
    /// units, then uses a total order. Callers resolve equal keys with
    /// [`Self::coplanar_order`]. Compare only depths from the same view/metric.
    /// This quantization is distinct from the coverage-plane coincidence test;
    /// an epsilon comparator would not provide a transitive ordering.
    pub fn compare_point_depths(left: f64, right: f64) -> std::cmp::Ordering {
        (left * 1e7).round().total_cmp(&(right * 1e7).round())
    }

    /// The part of `bounds` where `front` precedes `back` in a common projected coordinate chart.
    /// Coplanar paint uses the same facing/order key as the renderer and pointer tests.
    /// The corner depths use a `1e-7` coincidence tolerance in their shared view's
    /// depth units; crossing planes are split at their exact equality line.
    pub fn front_region(
        front: Self,
        back: Self,
        front_order: i32,
        back_order: i32,
        bounds: kurbo::Rect,
    ) -> Option<kurbo::BezPath> {
        if front.presentation_layer != back.presentation_layer {
            return (front.presentation_layer > back.presentation_layer).then(|| {
                use kurbo::Shape;
                bounds.to_path(0.1)
            });
        }
        let points = [
            kurbo::Point::new(bounds.x0, bounds.y0),
            kurbo::Point::new(bounds.x1, bounds.y0),
            kurbo::Point::new(bounds.x1, bounds.y1),
            kurbo::Point::new(bounds.x0, bounds.y1),
        ];
        let coplanar = front.coincides_over(back, points.map(|p| [p.x, p.y]));
        let mut result = Vec::new();
        if coplanar {
            let above =
                front.coplanar_order(front_order as i64) > back.coplanar_order(back_order as i64);
            if !above {
                return None;
            }
            result.extend(points);
        } else {
            let delta = points.map(|p| front.depth_at(p.x, p.y) - back.depth_at(p.x, p.y));
            for i in 0..4 {
                let j = (i + 1) % 4;
                if delta[i] >= 0.0 {
                    result.push(points[i]);
                }
                if (delta[i] > 0.0 && delta[j] < 0.0) || (delta[i] < 0.0 && delta[j] > 0.0) {
                    result.push(points[i].lerp(points[j], delta[i] / (delta[i] - delta[j])));
                }
            }
        }
        if result.len() < 3 {
            return None;
        }
        let mut path = kurbo::BezPath::new();
        path.move_to(result[0]);
        for p in &result[1..] {
            path.line_to(*p);
        }
        path.close_path();
        Some(path)
    }
}

/// The drawing contract shared by Pax runtime backends.
pub trait RenderContext {
    /// Supplies mounted/support identities and the runtime-owned relation cache.
    /// Retained replay carries these values without retaining reactive nodes.
    #[cfg(feature = "3d")]
    fn set_node_coverage_context(
        &mut self,
        _layer: usize,
        _node: u32,
        _context: crate::visibility::CoverageContext,
    ) {
    }
    /// Draws one opaque convex mesh. Only the opt-in GPU 3D backend supports this.
    #[cfg(feature = "3d")]
    fn draw_convex_mesh(&mut self, _layer: usize, _draw: crate::mesh3::OpaqueMeshDraw) {}
    /// Supplies the current node's local-to-world shading frame. Call after begin
    /// and before paint. Backends without material lighting may ignore it.
    #[cfg(feature = "3d")]
    fn set_node_lighting_surface(
        &mut self,
        _layer: usize,
        _node_id: u32,
        _surface: Option<LightingSurface>,
    ) {
    }
    /// Supplies retained planar depth for an orthographically projected node.
    /// `None` restores the ordinary 2D plane. Called after beginning the node.
    #[cfg(feature = "3d")]
    fn set_node_depth_plane(&mut self, _layer: usize, _node_id: u32, _plane: Option<DepthPlane>) {}
    /// Admit a rendering turn without waiting for GPU completion. Returning false
    /// leaves scene work dirty for a later turn; input and application ticks continue.
    /// `application` is the property graph identity and `frame` its tick epoch;
    /// cooperative slices of the same frame share its GPU allowance. Nested turns
    /// reuse the outer admission. Pair each call returning true with
    /// `end_render_turn`, including turns that discover no drawing work.
    fn begin_render_turn(&mut self, _application: u64, _frame: u64) -> bool {
        true
    }
    /// Finish an admitted turn. GPU backends fence its submissions together rather
    /// than spending a separate frame allowance for each canvas layer or tile.
    fn end_render_turn(&mut self) {}

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
    /// Begins an effect in a camera-projected coordinate system. The effect still
    /// samples in its authored local coordinates; camera clipping bounds its output.
    #[cfg(feature = "shaders")]
    fn begin_projected_shader_effect(
        &mut self,
        layer: usize,
        id: u32,
        z: i32,
        effect: &ShaderEffect,
        projection: crate::math::ProjectedPlane,
        bounds: (f64, f64),
        scopes: &[OpacityScope],
        children: &[u32],
    ) -> bool {
        self.begin_shader_effect(
            layer,
            id,
            z,
            effect,
            projection
                .mapping()
                .as_affine()
                .expect("this backend does not support projected effects"),
            bounds,
            scopes,
            children,
        )
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
    /// Applies a homogeneous flat-surface projection, including camera depth clipping.
    /// Perspective-capable backends must preserve w through rasterization.
    fn project(&mut self, _layer: usize, _plane: crate::math::ProjectedPlane) {
        panic!("this rendering backend does not support perspective projection");
    }

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
    /// Supply world-space camera lighting coordinates alongside layer lighting.
    /// `view.xyz` is an eye position when `view.w` is one, or a direction toward
    /// the viewer when zero. The default keeps software/legacy hosts compatible.
    fn set_scene_lighting_with_view(
        &mut self,
        layer: usize,
        lighting: &SceneLighting,
        surface_to_world: kurbo::Affine,
        _view: [f32; 4],
    ) {
        self.set_scene_lighting_with_transform(layer, lighting, surface_to_world);
    }

    /// Installs lighting with the canvas's unscrolled world frame. The affine
    /// frame keeps planar lighting independent of tiling and native scrolling.
    fn set_scene_lighting_with_transform(
        &mut self,
        layer: usize,
        lighting: &SceneLighting,
        _surface_to_world: kurbo::Affine,
    ) {
        self.set_scene_lighting(layer, lighting);
    }
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

    /// Updates retained paint order without recording drawing commands again.
    /// Returning true accepts the entire batch, including future replay of cached
    /// drawings. Numeric renumbering that preserves relative order need not repaint.
    /// Returning false requests the runtime's conservative redraw fallback.
    fn update_node_orders(&mut self, _layer: usize, _orders: &[(u32, i32)]) -> bool {
        false
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

#[cfg(test)]
mod depth_policy_tests {
    use super::DepthPlane;
    use kurbo::{Point, Rect, Shape};
    use std::cmp::Ordering;

    #[test]
    fn point_depth_quantization_preserves_boundaries_and_total_order() {
        let values = [-1.01e-7, -0.51e-7, -0.49e-7, 0., 0.49e-7, 0.51e-7, 1.01e-7];
        assert_eq!(
            DepthPlane::compare_point_depths(0., 0.49e-7),
            Ordering::Equal
        );
        assert_eq!(
            DepthPlane::compare_point_depths(0., 0.51e-7),
            Ordering::Less
        );
        for &a in &values {
            for &b in &values {
                assert_eq!(
                    DepthPlane::compare_point_depths(a, b),
                    DepthPlane::compare_point_depths(b, a).reverse()
                );
                for &c in &values {
                    if DepthPlane::compare_point_depths(a, b).is_le()
                        && DepthPlane::compare_point_depths(b, c).is_le()
                    {
                        assert!(DepthPlane::compare_point_depths(a, c).is_le());
                    }
                }
            }
        }
    }

    #[test]
    fn crossing_coverage_matches_analytic_depth_on_both_sides() {
        let back = DepthPlane::default();
        let bounds = Rect::new(-100., -100., 100., 100.);
        for coefficients in [[1., 0., 0.], [0., -2., 10.], [0.3, -0.7, 12.]] {
            let front = DepthPlane {
                coefficients,
                backface: false,
            };
            let path = DepthPlane::front_region(front, back, 0, 0, bounds).unwrap();
            for x in [-80., -30., 30., 80.] {
                for y in [-80., -30., 30., 80.] {
                    let z = front.depth_at(x, y);
                    if z != 0. {
                        assert_eq!(path.winding(Point::new(x, y)) != 0, z > 0.);
                    }
                }
            }
        }
    }

    #[test]
    fn coverage_coincidence_and_point_quantization_are_distinct_policies() {
        let back = DepthPlane::default();
        let front = DepthPlane {
            coefficients: [0., 0., 0.75e-7],
            backface: false,
        };
        let bounds = Rect::new(0., 0., 20., 20.);
        // Characterize the existing boundary difference; consolidation does not
        // silently change either renderer/mask coincidence or point-picking ties.
        assert!(DepthPlane::front_region(front, back, 0, 1, bounds).is_none());
        assert_eq!(
            DepthPlane::compare_point_depths(front.depth_at(10., 10.), 0.),
            Ordering::Greater
        );
    }
}
