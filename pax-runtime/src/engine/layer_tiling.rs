use serde::Serialize;

const SCROLLER_TARGET_TILE_BACKING_DIMENSION: f64 = 2496.0;
const MIN_LOGICAL_TILE_SIZE: f64 = 256.0;
const TILE_OVERSCAN_COLUMNS: i32 = 0;
const TILE_OVERSCAN_ROWS: i32 = 0;
const MIN_UNTILED_RENDER_DPR: f64 = 1.0;
const PREWARM_VIEWPORT_PAD_X_MULTIPLIER: f64 = 1.0;
const PREWARM_VIEWPORT_PAD_Y_MULTIPLIER: f64 = 1.0;
const PREWARM_VIEWPORT_PAD_MIN_X: f64 = 512.0;
const PREWARM_VIEWPORT_PAD_MIN_Y: f64 = 512.0;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceCanvasDescriptor {
    pub id: String,
    pub key: String,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub surface_signature: String,
    pub transform_signature: String,
    pub host_signature: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerCanvasPlan {
    pub layer_id: usize,
    pub active: bool,
    pub surfaces: Vec<SurfaceCanvasDescriptor>,
}

pub fn single_surface_plan(
    layer_id: usize,
    host_signature: String,
    width: f64,
    height: f64,
) -> LayerCanvasPlan {
    let width = width.max(0.0);
    let height = height.max(0.0);
    LayerCanvasPlan {
        layer_id,
        active: true,
        surfaces: vec![SurfaceCanvasDescriptor {
            id: layer_id.to_string(),
            key: "single".to_string(),
            left: 0.0,
            top: 0.0,
            width,
            height,
            surface_signature: format!("single:{width}x{height}@{host_signature}"),
            transform_signature: "0,0".to_string(),
            host_signature,
        }],
    }
}

pub fn scroller_canvas_plan(
    layer_id: usize,
    host_signature: String,
    content_width: f64,
    content_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    scroll_x: f64,
    scroll_y: f64,
    device_pixel_ratio: f64,
) -> LayerCanvasPlan {
    let content_width = content_width.max(0.0);
    let content_height = content_height.max(0.0);
    let viewport_width = clamp_dimension(viewport_width, content_width);
    let viewport_height = clamp_dimension(viewport_height, content_height);
    let scroll_x = clamp_offset(scroll_x, content_width, viewport_width);
    let scroll_y = clamp_offset(scroll_y, content_height, viewport_height);
    let dpr = device_pixel_ratio.max(1.0);

    if can_render_as_single_surface(content_width, content_height, dpr) {
        return single_surface_plan(layer_id, host_signature, content_width, content_height);
    }

    let tile_size = compute_logical_tile_size(dpr);
    if content_width <= tile_size && content_height <= tile_size {
        return single_surface_plan(layer_id, host_signature, content_width, content_height);
    }

    let max_column = ((content_width / tile_size).ceil() as i32 - 1).max(0);
    let max_row = ((content_height / tile_size).ceil() as i32 - 1).max(0);
    let horizontal_scrollable = content_width > viewport_width + 0.5;
    let vertical_scrollable = content_height > viewport_height + 0.5;
    let pad_x = if horizontal_scrollable {
        (viewport_width * PREWARM_VIEWPORT_PAD_X_MULTIPLIER).max(PREWARM_VIEWPORT_PAD_MIN_X)
    } else {
        0.0
    };
    let pad_y = if vertical_scrollable {
        (viewport_height * PREWARM_VIEWPORT_PAD_Y_MULTIPLIER).max(PREWARM_VIEWPORT_PAD_MIN_Y)
    } else {
        0.0
    };
    let padded_viewport_width = viewport_width + pad_x * 2.0;
    let padded_viewport_height = viewport_height + pad_y * 2.0;
    let padded_scroll_x = clamp_offset(scroll_x - pad_x, content_width, padded_viewport_width);
    let padded_scroll_y = clamp_offset(scroll_y - pad_y, content_height, padded_viewport_height);
    let overscan_columns = if horizontal_scrollable {
        TILE_OVERSCAN_COLUMNS
    } else {
        0
    };
    let overscan_rows = if vertical_scrollable {
        TILE_OVERSCAN_ROWS
    } else {
        0
    };
    let active_columns = (max_column + 1)
        .min(visible_tile_span(padded_viewport_width, tile_size, true) + overscan_columns * 2);
    let active_rows = (max_row + 1)
        .min(visible_tile_span(padded_viewport_height, tile_size, true) + overscan_rows * 2);
    let start_column = clamp_window_start(
        (padded_scroll_x / tile_size).floor() as i32 - overscan_columns,
        max_column,
        active_columns,
    );
    let start_row = clamp_window_start(
        (padded_scroll_y / tile_size).floor() as i32 - overscan_rows,
        max_row,
        active_rows,
    );
    let end_column = max_column.min(start_column + active_columns - 1);
    let end_row = max_row.min(start_row + active_rows - 1);

    let mut surfaces = Vec::new();
    for row in start_row..=end_row {
        for column in start_column..=end_column {
            let slot_column = column - start_column;
            let slot_row = row - start_row;
            let left = column as f64 * tile_size;
            let top = row as f64 * tile_size;
            let width = (content_width - left).min(tile_size).max(1.0);
            let height = (content_height - top).min(tile_size).max(1.0);
            let key = format!("{slot_column}:{slot_row}");
            let id = format!("layer-{layer_id}-tile-{slot_column}-{slot_row}");
            let surface_signature = format!(
                "tile:{slot_column},{slot_row}:{width}x{height}:{viewport_width}x{viewport_height}:{host_signature}"
            );
            let transform_signature = format!("{left},{top}");
            surfaces.push(SurfaceCanvasDescriptor {
                id,
                key,
                left,
                top,
                width,
                height,
                surface_signature,
                transform_signature,
                host_signature: host_signature.clone(),
            });
        }
    }

    LayerCanvasPlan {
        layer_id,
        active: true,
        surfaces,
    }
}

fn compute_logical_tile_size(device_pixel_ratio: f64) -> f64 {
    let tile_size = (SCROLLER_TARGET_TILE_BACKING_DIMENSION / device_pixel_ratio).floor();
    tile_size.max(MIN_LOGICAL_TILE_SIZE)
}

fn can_render_as_single_surface(content_width: f64, content_height: f64, desired_dpr: f64) -> bool {
    let safe_width = content_width.max(1.0);
    let safe_height = content_height.max(1.0);
    let backing_dimension = SCROLLER_TARGET_TILE_BACKING_DIMENSION.max(1.0);
    let single_surface_dpr = desired_dpr
        .min(backing_dimension / safe_width)
        .min(backing_dimension / safe_height);
    single_surface_dpr >= MIN_UNTILED_RENDER_DPR
}

fn clamp_dimension(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0).min(fallback.max(0.0))
    } else {
        fallback.max(0.0)
    }
}

fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
    if content <= viewport {
        return 0.0;
    }
    if !value.is_finite() {
        return 0.0;
    }
    value.max(0.0).min((content - viewport).max(0.0))
}

fn visible_tile_span(viewport_size: f64, tile_size: f64, include_extra_slot: bool) -> i32 {
    let base = (viewport_size.max(0.0) / tile_size).ceil() as i32;
    let extra = if include_extra_slot { 1 } else { 0 };
    (base + extra).max(1)
}

fn clamp_window_start(value: i32, max_index: i32, window_size: i32) -> i32 {
    clamp_index(value, (max_index - window_size + 1).max(0))
}

fn clamp_index(value: i32, max: i32) -> i32 {
    value.max(0).min(max)
}
