//! Tile-independent drawing programs. These contain values, never reactive Properties or
//! component callbacks. Surface replay executes a program without revisiting the component tree.

use super::pax_gpu_render_context::{
    convert_kurbo_to_lyon_path, to_pax_gpu_alpha_fill, to_pax_gpu_fill,
};
use kurbo::Rect;
use pax_gpu::{
    point, Box2D, DrawRange, Image, Material, Path, Stroke, StrokeCap, StrokeJoin, Transform2D,
    WgpuRenderer,
};
use pax_runtime_api::{AlphaMaskPaint, OpacityScope, Paint, PathSmoothing};
use std::{collections::HashMap, rc::Rc};

pub(super) enum Command {
    Begin(u32, i32, u32),
    End(u32),
    Save,
    Restore,
    Transform(Transform2D),
    Clip(Path),
    Alpha(Vec<AlphaMaskPaint>, f32),
    BeginSource(Transform2D, f32),
    EndSource,
    Opacity(u32, Vec<OpacityScope>),
    Fill {
        path: Path,
        bounds: Rect,
        paint: Paint,
        material: Material,
        opacity: f32,
        smoothing: PathSmoothing,
    },
    Stroke {
        path: Path,
        bounds: Rect,
        paint: Paint,
        material: Material,
        opacity: f32,
        width: f32,
        cap: StrokeCap,
        join: StrokeJoin,
        range: DrawRange,
        smoothing: PathSmoothing,
    },
    Image(String, Rect, f32),
}

impl Command {
    pub(super) fn execute(
        &self,
        gpu: &mut WgpuRenderer,
        images: &HashMap<String, Image>,
        versions: &HashMap<String, u64>,
    ) {
        match self {
            Self::Begin(id, z, lights) => {
                gpu.begin_node(*id, *z, *lights);
            }
            Self::End(id) => {
                gpu.end_node(*id);
            }
            Self::Save => gpu.save(),
            Self::Restore => gpu.restore(),
            Self::Transform(t) => gpu.transform(*t),
            Self::Clip(p) => gpu.clip(p.clone()),
            Self::Alpha(paints, feather) => gpu.clip_alpha(
                paints
                    .iter()
                    .map(|p| {
                        let transform =
                            Transform2D::from_array(p.transform.as_coeffs().map(|v| v as f32));
                        pax_gpu::AlphaMaskPaint {
                            path: convert_kurbo_to_lyon_path(&p.path),
                            fill: to_pax_gpu_alpha_fill(
                                &p.fill,
                                p.paint_bounds,
                                transform.then(&gpu.current_transform()),
                            ),
                            transform,
                            opacity: p.opacity.clamp(0.0, 1.0) as f32,
                            composition: p.composition,
                        }
                    })
                    .collect(),
                *feather,
            ),
            Self::BeginSource(t, feather) => gpu.begin_alpha_source(*t, *feather),
            Self::EndSource => gpu.end_alpha_source(),
            Self::Opacity(id, scopes) => gpu.set_node_opacity_scopes(*id, scopes),
            Self::Fill {
                path,
                bounds,
                paint,
                material,
                opacity,
                smoothing,
            } => {
                let fill = to_pax_gpu_fill(paint, *bounds, gpu.current_transform());
                gpu.fill_path_with_material_and_opacity_and_smoothing(
                    path.clone(),
                    fill,
                    *material,
                    *opacity,
                    *smoothing,
                );
            }
            Self::Stroke {
                path,
                bounds,
                paint,
                material,
                opacity,
                width,
                cap,
                join,
                range,
                smoothing,
            } => {
                let stroke = Stroke {
                    fill: to_pax_gpu_fill(paint, *bounds, gpu.current_transform()),
                    weight: *width,
                    cap: *cap,
                    join: *join,
                };
                gpu.stroke_path_with_draw_range_and_material_and_opacity_and_smoothing(
                    path.clone(),
                    stroke,
                    *material,
                    *opacity,
                    *range,
                    *smoothing,
                );
            }
            Self::Image(key, rect, opacity) => {
                if let Some(image) = images.get(key) {
                    gpu.draw_image_with_opacity(
                        key,
                        *versions.get(key).unwrap_or(&0),
                        image,
                        Box2D {
                            min: point(rect.x0 as f32, rect.y0 as f32),
                            max: point(rect.x1 as f32, rect.y1 as f32),
                        },
                        *opacity,
                    );
                }
            }
        }
    }

    // Weighted payload units bound both record count and large paths/paint fields. Prefixes
    // share Rc commands but are charged per record, conservatively, rather than undercounted.
    fn units(&self) -> usize {
        1 + match self {
            Self::Fill { path, paint, .. } | Self::Stroke { path, paint, .. } => {
                path.iter().count() + paint_units(paint)
            }
            Self::Clip(path) => path.iter().count(),
            Self::Alpha(paints, _) => paints
                .iter()
                .map(|p| p.path.elements().len() + paint_units(&p.fill) + 1)
                .sum(),
            Self::Opacity(_, scopes) => scopes.len(),
            Self::Image(key, ..) => key.len(),
            _ => 0,
        }
    }
}

fn paint_units(paint: &Paint) -> usize {
    1 + match paint {
        Paint::Solid(_) => 0,
        Paint::LinearGradient(p) => p.stops.len(),
        Paint::RadialGradient(p) => p.stops.len(),
        Paint::MeshGradient(p) => p.rows.iter().map(Vec::len).sum(),
        Paint::Blend(paints) => paints.iter().map(|(p, _)| paint_units(p)).sum(),
    }
}

pub(super) struct Drawing {
    pub bounds: Rect,
    pub commands: Vec<Rc<Command>>,
    units: usize,
}

struct Pending {
    id: u32,
    bounds: Option<Rect>,
    start: usize,
    save_depth: usize,
    commands: Vec<Rc<Command>>,
}

#[derive(Default)]
struct Recording {
    state: Vec<Rc<Command>>,
    saves: Vec<usize>,
    pending: Vec<Pending>,
    source_depth: usize,
}

#[derive(Default)]
pub(super) struct DrawingCache {
    recordings: HashMap<usize, Recording>,
    drawings: HashMap<(usize, u32), (Rc<Drawing>, u64)>,
    clock: u64,
    units: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl DrawingCache {
    const MAX_UNITS: usize = 1_000_000;
    const MAX_RECORDS: usize = 4096;

    pub fn begin(&mut self, layer: usize, id: u32, z: i32, lights: u32, bounds: Option<Rect>) {
        self.remove(layer, id);
        let r = self.recordings.entry(layer).or_default();
        r.pending.push(Pending {
            id,
            bounds: bounds.filter(|_| r.source_depth == 0),
            start: r.state.len(),
            save_depth: r.saves.len(),
            commands: r.state.clone(),
        });
        self.push(layer, Rc::new(Command::Begin(id, z, lights)));
    }

    pub fn push(&mut self, layer: usize, command: Rc<Command>) {
        let r = self.recordings.entry(layer).or_default();
        for pending in &mut r.pending {
            pending.commands.push(Rc::clone(&command));
        }
        match command.as_ref() {
            Command::Save => {
                // Frame/Mask begin a node before saving the clip state. Restoring that state
                // must also discard its bookkeeping Begin, not leave it in a leaf's prefix.
                let checkpoint = r
                    .pending
                    .last()
                    .filter(|p| r.state.len() == p.start + 1)
                    .map_or(r.state.len(), |p| p.start);
                r.saves.push(checkpoint);
            }
            Command::Restore => {
                if let Some(checkpoint) = r.saves.pop() {
                    r.state.truncate(checkpoint);
                }
                return;
            }
            Command::BeginSource(..) => r.source_depth += 1,
            Command::EndSource => r.source_depth = r.source_depth.saturating_sub(1),
            _ => {}
        }
        r.state.push(command);
    }

    pub fn end(&mut self, layer: usize, id: u32) {
        self.push(layer, Rc::new(Command::End(id)));
        let r = self.recordings.get_mut(&layer).unwrap();
        let Some(pending) = r.pending.pop() else {
            return;
        };
        assert_eq!(pending.id, id, "unbalanced logical drawing node");
        if r.saves.len() == pending.save_depth {
            r.state.truncate(pending.start);
        } else {
            // A clip owner persists after end_node. Preserve its complete program, including
            // detached source draws whose own save/restore pairs have already unwound.
            r.state = pending.commands.clone();
        }
        if let Some(bounds) = pending.bounds {
            let units = pending.commands.iter().map(|c| c.units()).sum();
            if units <= Self::MAX_UNITS {
                self.clock += 1;
                self.units += units;
                self.drawings.insert(
                    (layer, id),
                    (
                        Rc::new(Drawing {
                            bounds,
                            commands: pending.commands,
                            units,
                        }),
                        self.clock,
                    ),
                );
                self.evict();
            }
        }
    }

    fn evict(&mut self) {
        while self.units > Self::MAX_UNITS || self.drawings.len() > Self::MAX_RECORDS {
            let Some((&key, _)) = self.drawings.iter().min_by_key(|(_, (_, used))| used) else {
                break;
            };
            self.remove(key.0, key.1);
            self.evictions += 1;
        }
    }

    pub fn get(&mut self, layer: usize, id: u32) -> Option<Rc<Drawing>> {
        self.clock += 1;
        if let Some((drawing, used)) = self.drawings.get_mut(&(layer, id)) {
            *used = self.clock;
            self.hits += 1;
            Some(Rc::clone(drawing))
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn remove(&mut self, layer: usize, id: u32) {
        if let Some((drawing, _)) = self.drawings.remove(&(layer, id)) {
            self.units -= drawing.units;
        }
    }

    pub fn finish_frame(&mut self, layer: usize) {
        self.recordings.remove(&layer);
    }

    pub fn truncate_layers(&mut self, count: usize) {
        self.drawings.retain(|(layer, _), _| *layer < count);
        self.units = self.drawings.values().map(|(d, _)| d.units).sum();
        self.recordings.retain(|layer, _| *layer < count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(cache: &mut DrawingCache, id: u32) {
        cache.begin(0, id, 0, 0, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
        cache.push(0, Rc::new(Command::Save));
        cache.push(0, Rc::new(Command::Image("test".into(), Rect::ZERO, 1.0)));
        cache.push(0, Rc::new(Command::Restore));
        cache.end(0, id);
    }

    #[test]
    fn cached_leaf_keeps_ancestor_clip_but_not_previous_sibling() {
        let mut cache = DrawingCache::default();
        cache.begin(0, 10, 0, 0, None);
        cache.push(0, Rc::new(Command::Save));
        cache.push(0, Rc::new(Command::Clip(Path::builder().build())));
        cache.end(0, 10);
        leaf(&mut cache, 11);
        leaf(&mut cache, 12);
        let drawing = cache.get(0, 12).unwrap();
        let ids: Vec<_> = drawing
            .commands
            .iter()
            .filter_map(|c| match c.as_ref() {
                Command::Begin(id, ..) => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(ids, [10, 12]);
        cache.push(0, Rc::new(Command::Restore));
        leaf(&mut cache, 13);
        assert!(matches!(
            cache.get(0, 13).unwrap().commands[0].as_ref(),
            Command::Begin(13, ..)
        ));
        cache.finish_frame(0);
        assert!(
            cache.get(0, 12).is_some(),
            "surface turnover must not evict the drawing"
        );
    }

    #[test]
    fn detached_mask_source_is_preserved_as_part_of_clip_prefix() {
        let mut cache = DrawingCache::default();
        cache.begin(0, 10, 0, 0, None);
        cache.push(0, Rc::new(Command::Save));
        cache.push(
            0,
            Rc::new(Command::BeginSource(Transform2D::identity(), 0.0)),
        );
        leaf(&mut cache, 20);
        cache.push(0, Rc::new(Command::EndSource));
        cache.end(0, 10);
        leaf(&mut cache, 11);
        let drawing = cache.get(0, 11).unwrap();
        let ids: Vec<_> = drawing
            .commands
            .iter()
            .filter_map(|c| match c.as_ref() {
                Command::Begin(id, ..) => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(ids, [10, 20, 11]);
        assert!(
            cache.get(0, 20).is_none(),
            "source cannot replay without its owner"
        );
    }

    #[test]
    fn cache_is_bounded_and_removal_and_layer_reuse_drop_old_drawings() {
        let mut cache = DrawingCache::default();
        for id in 0..DrawingCache::MAX_RECORDS as u32 + 2 {
            leaf(&mut cache, id);
        }
        assert_eq!(cache.drawings.len(), DrawingCache::MAX_RECORDS);
        assert!(cache.get(0, 0).is_none());
        cache.remove(0, 10);
        assert!(cache.get(0, 10).is_none());
        cache.truncate_layers(0);
        assert_eq!(cache.units, 0);
        assert!(cache.drawings.is_empty());
    }
}
