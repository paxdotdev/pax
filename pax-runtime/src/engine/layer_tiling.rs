use serde::Serialize;

const DEFAULT_SCROLLER_TARGET_TILE_BACKING_DIMENSION: f64 = 2496.0;
const DEFAULT_MIN_LOGICAL_TILE_SIZE: f64 = 256.0;
const DEFAULT_TILE_OVERSCAN_COLUMNS: i32 = 0;
const DEFAULT_TILE_OVERSCAN_ROWS: i32 = 0;
const DEFAULT_MIN_UNTILED_RENDER_DPR: f64 = 1.0;
const DEFAULT_PREWARM_VIEWPORT_PAD_X_MULTIPLIER: f64 = 1.0;
const DEFAULT_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER: f64 = 1.5;
const DEFAULT_PREWARM_VIEWPORT_PAD_MIN_X: f64 = 512.0;
const DEFAULT_PREWARM_VIEWPORT_PAD_MIN_Y: f64 = 512.0;

#[derive(Clone, Copy, Debug)]
pub struct ScrollerTilingPolicy {
    pub target_tile_backing_dimension: f64,
    pub min_logical_tile_size: f64,
    pub tile_overscan_columns: i32,
    pub tile_overscan_rows: i32,
    pub min_untiled_render_dpr: f64,
    pub prewarm_viewport_pad_x_multiplier: f64,
    pub prewarm_viewport_pad_y_multiplier: f64,
    pub prewarm_viewport_pad_min_x: f64,
    pub prewarm_viewport_pad_min_y: f64,
    pub max_surfaces_per_layer: Option<usize>,
}

impl Default for ScrollerTilingPolicy {
    fn default() -> Self {
        Self {
            target_tile_backing_dimension: DEFAULT_SCROLLER_TARGET_TILE_BACKING_DIMENSION,
            min_logical_tile_size: DEFAULT_MIN_LOGICAL_TILE_SIZE,
            tile_overscan_columns: DEFAULT_TILE_OVERSCAN_COLUMNS,
            tile_overscan_rows: DEFAULT_TILE_OVERSCAN_ROWS,
            min_untiled_render_dpr: DEFAULT_MIN_UNTILED_RENDER_DPR,
            prewarm_viewport_pad_x_multiplier: DEFAULT_PREWARM_VIEWPORT_PAD_X_MULTIPLIER,
            prewarm_viewport_pad_y_multiplier: DEFAULT_PREWARM_VIEWPORT_PAD_Y_MULTIPLIER,
            prewarm_viewport_pad_min_x: DEFAULT_PREWARM_VIEWPORT_PAD_MIN_X,
            prewarm_viewport_pad_min_y: DEFAULT_PREWARM_VIEWPORT_PAD_MIN_Y,
            max_surfaces_per_layer: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// One physical canvas surface used to render a logical Pax layer tile.
pub struct SurfaceCanvasDescriptor {
    pub id: String,
    pub key: String,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub replay_priority: i32,
    pub surface_signature: String,
    pub transform_signature: String,
    pub host_signature: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// Canvas tiling plan for one logical occlusion layer.
pub struct LayerCanvasPlan {
    pub layer_id: usize,
    pub active: bool,
    pub surfaces: Vec<SurfaceCanvasDescriptor>,
}

/// Build a one-surface plan for layers that do not need tiling.
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
            replay_priority: 0,
            surface_signature: format!("single:{width}x{height}@{host_signature}"),
            transform_signature: "0,0".to_string(),
            host_signature,
        }],
    }
}

/// Build a tile window for a scrollable vector layer.
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
    scroller_canvas_plan_with_policy(
        layer_id,
        host_signature,
        content_width,
        content_height,
        viewport_width,
        viewport_height,
        scroll_x,
        scroll_y,
        device_pixel_ratio,
        ScrollerTilingPolicy::default(),
    )
}

pub fn scroller_canvas_plan_with_policy(
    layer_id: usize,
    host_signature: String,
    content_width: f64,
    content_height: f64,
    viewport_width: f64,
    viewport_height: f64,
    scroll_x: f64,
    scroll_y: f64,
    device_pixel_ratio: f64,
    policy: ScrollerTilingPolicy,
) -> LayerCanvasPlan {
    let content_width = content_width.max(0.0);
    let content_height = content_height.max(0.0);
    let viewport_width = clamp_dimension(viewport_width, content_width);
    let viewport_height = clamp_dimension(viewport_height, content_height);
    let scroll_x = clamp_offset(scroll_x, content_width, viewport_width);
    let scroll_y = clamp_offset(scroll_y, content_height, viewport_height);
    let dpr = device_pixel_ratio.max(1.0);

    if can_render_as_single_surface(content_width, content_height, dpr, policy) {
        return single_surface_plan(layer_id, host_signature, content_width, content_height);
    }

    let tile_size = compute_logical_tile_size(dpr, policy);
    if content_width <= tile_size && content_height <= tile_size {
        return single_surface_plan(layer_id, host_signature, content_width, content_height);
    }

    let max_column = ((content_width / tile_size).ceil() as i32 - 1).max(0);
    let max_row = ((content_height / tile_size).ceil() as i32 - 1).max(0);
    let horizontal_scrollable = content_width > viewport_width + 0.5;
    let vertical_scrollable = content_height > viewport_height + 0.5;
    let pad_x = if horizontal_scrollable {
        (viewport_width * policy.prewarm_viewport_pad_x_multiplier)
            .max(policy.prewarm_viewport_pad_min_x)
    } else {
        0.0
    };
    let pad_y = if vertical_scrollable {
        (viewport_height * policy.prewarm_viewport_pad_y_multiplier)
            .max(policy.prewarm_viewport_pad_min_y)
    } else {
        0.0
    };
    let padded_viewport_width = viewport_width + pad_x * 2.0;
    let padded_viewport_height = viewport_height + pad_y * 2.0;
    let padded_scroll_x = clamp_offset(scroll_x - pad_x, content_width, padded_viewport_width);
    let padded_scroll_y = clamp_offset(scroll_y - pad_y, content_height, padded_viewport_height);
    let overscan_columns = if horizontal_scrollable {
        policy.tile_overscan_columns.max(0)
    } else {
        0
    };
    let overscan_rows = if vertical_scrollable {
        policy.tile_overscan_rows.max(0)
    } else {
        0
    };
    let active_columns = (max_column + 1)
        .min(visible_tile_span(padded_viewport_width, tile_size, true) + overscan_columns * 2);
    let active_rows = (max_row + 1)
        .min(visible_tile_span(padded_viewport_height, tile_size, true) + overscan_rows * 2);
    let min_active_columns =
        (max_column + 1).min(visible_tile_span(viewport_width, tile_size, true));
    let min_active_rows = (max_row + 1).min(visible_tile_span(viewport_height, tile_size, true));
    let (active_columns, active_rows) = clamp_active_tile_window(
        active_columns,
        active_rows,
        min_active_columns,
        min_active_rows,
        horizontal_scrollable,
        vertical_scrollable,
        policy.max_surfaces_per_layer,
    );
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
            let slot_column = column.rem_euclid(active_columns);
            let slot_row = row.rem_euclid(active_rows);
            let left = column as f64 * tile_size;
            let top = row as f64 * tile_size;
            let width = (content_width - left).min(tile_size).max(1.0);
            let height = (content_height - top).min(tile_size).max(1.0);
            let replay_priority = tile_replay_priority(
                left,
                top,
                width,
                height,
                scroll_x,
                scroll_y,
                viewport_width,
                viewport_height,
                tile_size,
            );
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
                replay_priority,
                surface_signature,
                transform_signature,
                host_signature: host_signature.clone(),
            });
        }
    }
    surfaces.sort_by(|left, right| {
        left.key
            .cmp(&right.key)
            .then_with(|| left.transform_signature.cmp(&right.transform_signature))
    });

    LayerCanvasPlan {
        layer_id,
        active: true,
        surfaces,
    }
}

fn compute_logical_tile_size(device_pixel_ratio: f64, policy: ScrollerTilingPolicy) -> f64 {
    let tile_size = (policy.target_tile_backing_dimension.max(1.0) / device_pixel_ratio).floor();
    tile_size.max(policy.min_logical_tile_size.max(1.0))
}

fn can_render_as_single_surface(
    content_width: f64,
    content_height: f64,
    desired_dpr: f64,
    policy: ScrollerTilingPolicy,
) -> bool {
    let safe_width = content_width.max(1.0);
    let safe_height = content_height.max(1.0);
    let backing_dimension = policy.target_tile_backing_dimension.max(1.0);
    let single_surface_dpr = desired_dpr
        .min(backing_dimension / safe_width)
        .min(backing_dimension / safe_height);
    single_surface_dpr >= policy.min_untiled_render_dpr.max(0.0)
}

fn tile_replay_priority(
    tile_left: f64,
    tile_top: f64,
    tile_width: f64,
    tile_height: f64,
    viewport_left: f64,
    viewport_top: f64,
    viewport_width: f64,
    viewport_height: f64,
    tile_size: f64,
) -> i32 {
    let tile_right = tile_left + tile_width;
    let tile_bottom = tile_top + tile_height;
    let viewport_right = viewport_left + viewport_width;
    let viewport_bottom = viewport_top + viewport_height;

    let intersects = tile_right >= viewport_left
        && tile_left <= viewport_right
        && tile_bottom >= viewport_top
        && tile_top <= viewport_bottom;
    if intersects {
        return 0;
    }

    let dx = if tile_right < viewport_left {
        viewport_left - tile_right
    } else if tile_left > viewport_right {
        tile_left - viewport_right
    } else {
        0.0
    };
    let dy = if tile_bottom < viewport_top {
        viewport_top - tile_bottom
    } else if tile_top > viewport_bottom {
        tile_top - viewport_bottom
    } else {
        0.0
    };

    ((dx.max(dy) / tile_size.max(1.0)).ceil() as i32 + 1).max(1)
}

fn clamp_active_tile_window(
    mut active_columns: i32,
    mut active_rows: i32,
    min_active_columns: i32,
    min_active_rows: i32,
    horizontal_scrollable: bool,
    vertical_scrollable: bool,
    max_surfaces: Option<usize>,
) -> (i32, i32) {
    let Some(max_surfaces) = max_surfaces else {
        return (active_columns.max(1), active_rows.max(1));
    };
    let max_surfaces = max_surfaces.max(1) as i32;

    // First trim only the axis that can actually scroll. This preserves visible-axis coverage
    // whenever the platform cap is large enough to fit the viewport tile span.
    while active_columns * active_rows > max_surfaces {
        let can_trim_rows = vertical_scrollable && active_rows > min_active_rows.max(1);
        let can_trim_columns = horizontal_scrollable && active_columns > min_active_columns.max(1);
        if can_trim_rows && (!can_trim_columns || active_rows >= active_columns) {
            active_rows -= 1;
            continue;
        }
        if can_trim_columns {
            active_columns -= 1;
            continue;
        }
        if can_trim_rows {
            active_rows -= 1;
            continue;
        }
        break;
    }

    // A too-small platform cap is a hard constraint. Fall back to shrinking whichever dimension
    // is larger rather than allowing a plan the chassis cannot materialize.
    while active_columns * active_rows > max_surfaces && (active_columns > 1 || active_rows > 1) {
        if active_rows >= active_columns && active_rows > 1 {
            active_rows -= 1;
        } else if active_columns > 1 {
            active_columns -= 1;
        } else {
            active_rows -= 1;
        }
    }

    (active_columns.max(1), active_rows.max(1))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_only_scroller_does_not_add_horizontal_overscan() {
        let mut policy = ScrollerTilingPolicy::default();
        policy.tile_overscan_columns = 3;
        policy.tile_overscan_rows = 0;
        let plan = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            10_000.0,
            600.0,
            600.0,
            0.0,
            0.0,
            1.0,
            policy,
        );

        assert!(plan.surfaces.iter().all(|surface| surface.left == 0.0));
    }

    #[test]
    fn vertical_overscan_is_axis_gated() {
        let mut policy = ScrollerTilingPolicy::default();
        policy.tile_overscan_rows = 1;
        let vertical_plan = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            10_000.0,
            600.0,
            600.0,
            0.0,
            0.0,
            1.0,
            policy,
        );
        let horizontal_plan = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            10_000.0,
            600.0,
            600.0,
            600.0,
            0.0,
            0.0,
            1.0,
            policy,
        );

        assert!(vertical_plan.surfaces.len() > horizontal_plan.surfaces.len());
    }

    #[test]
    fn max_surfaces_clamps_requested_tile_window() {
        let mut policy = ScrollerTilingPolicy::default();
        policy.tile_overscan_rows = 2;
        policy.max_surfaces_per_layer = Some(3);
        let plan = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            20_000.0,
            600.0,
            600.0,
            0.0,
            5_000.0,
            1.0,
            policy,
        );

        assert!(plan.surfaces.len() <= 3);
    }

    #[test]
    fn overlapping_vertical_tile_keeps_physical_slot_key() {
        let policy = ScrollerTilingPolicy::default();
        let first = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            20_000.0,
            600.0,
            600.0,
            0.0,
            0.0,
            1.0,
            policy,
        );
        let second = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            20_000.0,
            600.0,
            600.0,
            0.0,
            3_200.0,
            1.0,
            policy,
        );

        let overlapping_surface = first
            .surfaces
            .iter()
            .find(|surface| surface.top > 0.0)
            .expect("first plan should have a second row");
        let matching_surface = second
            .surfaces
            .iter()
            .find(|surface| surface.top == overlapping_surface.top)
            .expect("second plan should keep the overlapping row warm");

        assert_eq!(matching_surface.key, overlapping_surface.key);
        assert_eq!(matching_surface.id, overlapping_surface.id);
    }

    #[test]
    fn replay_priority_prefers_visible_tiles() {
        let policy = ScrollerTilingPolicy::default();
        let plan = scroller_canvas_plan_with_policy(
            1,
            "test".to_string(),
            600.0,
            20_000.0,
            600.0,
            600.0,
            0.0,
            5_000.0,
            1.0,
            policy,
        );

        let visible_tiles = plan
            .surfaces
            .iter()
            .filter(|surface| surface.top + surface.height >= 5_000.0 && surface.top <= 5_600.0)
            .count();
        assert!(visible_tiles > 0);
        assert!(plan
            .surfaces
            .iter()
            .filter(|surface| surface.top + surface.height >= 5_000.0 && surface.top <= 5_600.0)
            .all(|surface| surface.replay_priority == 0));
        assert!(plan
            .surfaces
            .iter()
            .any(|surface| surface.replay_priority > 0));
    }
}
