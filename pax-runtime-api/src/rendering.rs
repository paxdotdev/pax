//! Rendering backend contracts and helpers for drawing Pax scene content.

use super::*;

/// Replay invalidation for one logical canvas layer.
#[derive(Clone, Debug)]
pub struct ReplayCanvasLayerUpdate {
    pub layer: usize,
    /// `None` means the layer should fall back to region/full-layer dirtification.
    pub node_ids: Option<Vec<u32>>,
}

/// The Pax render trait, used as a layer of indirection and contract for backend-agnostic rendering.
pub trait RenderContext {
    // Drawing
    /// Fills a path at full opacity.
    fn fill(&mut self, layer: usize, path: kurbo::BezPath, fill: &Fill) {
        self.fill_with_opacity(layer, path, fill, 1.0);
    }
    /// Fills a path with an explicit opacity multiplier.
    fn fill_with_opacity(&mut self, layer: usize, path: kurbo::BezPath, fill: &Fill, opacity: f64);
    /// Fills a path with a material and explicit opacity multiplier.
    fn fill_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        fill: &Fill,
        _material: &Material,
        opacity: f64,
    ) {
        self.fill_with_opacity(layer, path, fill, opacity);
    }
    /// Strokes a path at full opacity.
    fn stroke(&mut self, layer: usize, path: kurbo::BezPath, stroke: &Stroke) {
        self.stroke_with_opacity(layer, path, stroke, 1.0);
    }
    /// Strokes a path with an explicit opacity multiplier.
    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        opacity: f64,
    );
    /// Strokes a path with a material and explicit opacity multiplier.
    fn stroke_with_material_and_opacity(
        &mut self,
        layer: usize,
        path: kurbo::BezPath,
        stroke: &Stroke,
        _material: &Material,
        opacity: f64,
    ) {
        self.stroke_with_opacity(layer, path, stroke, opacity);
    }

    // Clip/transform
    /// Saves the current graphics state for a layer.
    fn save(&mut self, layer: usize);
    /// Restores the previous graphics state for a layer.
    fn restore(&mut self, layer: usize);
    /// Applies a clipping path to a layer.
    fn clip(&mut self, layer: usize, path: BezPath);
    /// Applies an affine transform to a layer.
    fn transform(&mut self, layer: usize, affine: kurbo::Affine);

    // Images
    /// Loads raw RGBA image bytes under an identifier.
    fn load_image(&mut self, identifier: &str, image: &[u8], width: usize, height: usize);
    /// Draws a previously loaded image into `rect`.
    fn draw_image(&mut self, layer: usize, image_path: &str, rect: kurbo::Rect);
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
    /// Returns canvas layers ready to present.
    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        vec![]
    }

    /// Returns canvas layers that should replay prior draw commands.
    fn take_replay_canvas_layers(&mut self) -> Vec<usize> {
        vec![]
    }

    /// Returns canvas layer replay work, optionally narrowed to exact retained node ids.
    fn take_replay_canvas_layer_updates(&mut self) -> Vec<ReplayCanvasLayerUpdate> {
        self.take_replay_canvas_layers()
            .into_iter()
            .map(|layer| ReplayCanvasLayerUpdate {
                layer,
                node_ids: None,
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

    /// Begins rendering a node and returns false when the backend can skip it.
    fn begin_node(&mut self, _layer: usize, _node_id: u32, _z_index: i32) -> bool {
        true
    }

    /// Begins rendering a node with conservative canvas-space coverage bounds.
    fn begin_node_with_bounds(
        &mut self,
        layer: usize,
        node_id: u32,
        z_index: i32,
        _coverage_bounds: kurbo::Rect,
    ) -> bool {
        self.begin_node(layer, node_id, z_index)
    }

    /// Returns true when a bounded node skipped by `begin_node_with_bounds` is clean for now.
    fn take_clean_skipped_node(&mut self, _layer: usize, _node_id: u32) -> bool {
        false
    }

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
