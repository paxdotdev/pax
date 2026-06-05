#![allow(unused_imports)]

use pax_kit::*;
use std::{f64::consts::PI, rc::Rc};

const PANEL_COUNT: f64 = 16.0;
const DESKTOP_PANEL_HEIGHT: f64 = 230.0;
const COMPACT_PANEL_HEIGHT: f64 = 390.0;
const COMPACT_WIDTH_THRESHOLD: f64 = 600.0;
const PANEL_GUTTER: f64 = 15.0;
const LIGHT_MARKER_SIZE: f64 = 34.0;
const FIRST_BASELINE_SPAN_MIN: f64 = 0.08;
const FIRST_BASELINE_SPAN_MAX: f64 = 0.15;
const HALF_WAVE_SPAN_MIN: f64 = 0.09;
const HALF_WAVE_SPAN_MAX: f64 = 0.20;
const END_SETTLE_SPAN_MIN: f64 = 0.08;
const END_SETTLE_SPAN_MAX: f64 = 0.18;
const WAVE_AMPLITUDE_MIN: f64 = 0.16;
const WAVE_AMPLITUDE_MAX: f64 = 0.48;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub scroll_y: Property<f64>,
    pub is_compact: Property<bool>,
    pub panel_height: Property<f64>,
    pub content_height: Property<f64>,
    pub scroll_progress: Property<f64>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub material_00: Property<Material>,
    pub material_01: Property<Material>,
    pub material_02: Property<Material>,
    pub material_03: Property<Material>,
    pub material_04: Property<Material>,
    pub material_05: Property<Material>,
    pub material_06: Property<Material>,
    pub material_07: Property<Material>,
    pub material_08: Property<Material>,
    pub material_09: Property<Material>,
    pub material_10: Property<Material>,
    pub material_11: Property<Material>,
    pub material_12: Property<Material>,
    pub material_13: Property<Material>,
    pub material_14: Property<Material>,
    pub material_15: Property<Material>,
    pub background_material: Property<Material>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.scroll_y.set(0.0);
        self.install_materials();
        let wave_path = Rc::new(std::cell::RefCell::new(LightWavePath::new(
            ctx.elapsed_time_millis() as u64,
        )));

        let bounds_for_compact = ctx.bounds_self.clone();
        let bounds_for_compact_calc = bounds_for_compact.clone();
        self.is_compact.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_compact_calc.get();
                is_compact_width(width)
            },
            &[bounds_for_compact.untyped()],
        ));

        let bounds_for_panel_height = ctx.bounds_self.clone();
        let bounds_for_panel_height_calc = bounds_for_panel_height.clone();
        self.panel_height.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_panel_height_calc.get();
                panel_height_for_width(width)
            },
            &[bounds_for_panel_height.untyped()],
        ));

        let bounds_for_content_height = ctx.bounds_self.clone();
        let bounds_for_content_height_calc = bounds_for_content_height.clone();
        self.content_height.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_for_content_height_calc.get();
                content_height_for_width(width)
            },
            &[bounds_for_content_height.untyped()],
        ));

        let bounds_for_progress = ctx.bounds_self.clone();
        let bounds_for_progress_calc = bounds_for_progress.clone();
        let scroll_y_for_progress = self.scroll_y.clone();
        let scroll_y_for_progress_calc = scroll_y_for_progress.clone();
        self.scroll_progress.replace_with(Property::computed(
            move || {
                let (width, height) = bounds_for_progress_calc.get();
                scroll_progress(width, height, scroll_y_for_progress_calc.get())
            },
            &[
                bounds_for_progress.untyped(),
                scroll_y_for_progress.untyped(),
            ],
        ));

        let bounds_for_x = ctx.bounds_self.clone();
        let bounds_for_x_calc = bounds_for_x.clone();
        let scroll_y_for_x = self.scroll_y.clone();
        let scroll_y_for_x_calc = scroll_y_for_x.clone();
        let wave_path_for_x = Rc::clone(&wave_path);
        self.light_x.replace_with(Property::computed(
            move || {
                let (width, height) = bounds_for_x_calc.get();
                let (x, _) =
                    wave_path_for_x
                        .borrow_mut()
                        .position(width, height, scroll_y_for_x_calc.get());
                x
            },
            &[bounds_for_x.untyped(), scroll_y_for_x.untyped()],
        ));

        let bounds_for_y = ctx.bounds_self.clone();
        let bounds_for_y_calc = bounds_for_y.clone();
        let scroll_y_for_y = self.scroll_y.clone();
        let scroll_y_for_y_calc = scroll_y_for_y.clone();
        let wave_path_for_y = Rc::clone(&wave_path);
        self.light_y.replace_with(Property::computed(
            move || {
                let (width, height) = bounds_for_y_calc.get();
                let (_, y) =
                    wave_path_for_y
                        .borrow_mut()
                        .position(width, height, scroll_y_for_y_calc.get());
                y
            },
            &[bounds_for_y.untyped(), scroll_y_for_y.untyped()],
        ));
    }

    fn install_materials(&mut self) {
        self.material_00.set(Material::matte());
        self.material_01.set(Material::matte());
        self.material_02.set(Material::glossy(0.22));
        self.material_03.set(Material::glossy(0.68));
        self.material_04.set(Material::metallic(0.34));
        self.material_05.set(Material::metallic(0.86));
        self.material_06
            .set(Material::emissive(Color::from_hex("FFD49A"), 0.18));
        self.material_07
            .set(Material::emissive(Color::from_hex("8FD4FF"), 0.28));
        self.material_08
            .set(custom_material(0.92, 0.66, 0.02, 0.98, 0.0, "000000", 0.0));
        self.material_09
            .set(custom_material(0.44, 0.78, 0.82, 0.14, 0.0, "000000", 0.0));
        self.material_10
            .set(custom_material(0.56, 0.52, 0.72, 0.22, 0.18, "000000", 0.0));
        self.material_11
            .set(custom_material(0.42, 0.42, 0.62, 0.46, 0.82, "000000", 0.0));
        self.material_12
            .set(custom_material(1.00, 0.72, 0.00, 1.00, 0.0, "000000", 0.0));
        self.material_13
            .set(custom_material(0.32, 0.70, 1.00, 0.06, 0.0, "000000", 0.0));
        self.material_14
            .set(Material::emissive(Color::from_hex("D2A7FF"), 0.36));
        self.material_15.set(Material::glossy(0.44));
        self.background_material.set(Material::unlit());
    }
}

fn is_compact_width(width: f64) -> bool {
    width < COMPACT_WIDTH_THRESHOLD
}

fn panel_height_for_width(width: f64) -> f64 {
    if is_compact_width(width) {
        COMPACT_PANEL_HEIGHT
    } else {
        DESKTOP_PANEL_HEIGHT
    }
}

fn content_height_for_width(width: f64) -> f64 {
    (PANEL_COUNT * panel_height_for_width(width)) + ((PANEL_COUNT + 1.0) * PANEL_GUTTER)
}

fn scroll_progress(viewport_width: f64, viewport_height: f64, scroll_y: f64) -> f64 {
    let max_scroll = (content_height_for_width(viewport_width) - viewport_height.max(1.0)).max(1.0);
    (scroll_y / max_scroll).clamp(0.0, 1.0)
}

struct LightWavePath {
    seed: u64,
    covered_progress: f64,
    next_side: f64,
    segments: Vec<WaveSegment>,
}

#[derive(Clone, Copy)]
enum WaveSegment {
    EdgeToBaseline {
        start: f64,
        end: f64,
    },
    Lobe {
        start: f64,
        end: f64,
        amplitude: f64,
        side: f64,
    },
    BaselineToEdge {
        start: f64,
        end: f64,
    },
}

impl LightWavePath {
    fn new(seed: u64) -> Self {
        Self {
            seed: seed ^ 0xa076_1d64_78bd_642f,
            covered_progress: 0.0,
            next_side: 1.0,
            segments: Vec::new(),
        }
    }

    fn position(&mut self, width: f64, height: f64, scroll_y: f64) -> (f64, f64) {
        let width = width.max(1.0);
        let height = height.max(1.0);
        let x_range = (width - LIGHT_MARKER_SIZE).max(0.0);
        let y_range = (height - LIGHT_MARKER_SIZE).max(0.0);
        let progress = scroll_progress(width, height, scroll_y);

        if progress <= 0.0 {
            return (0.0, 0.0);
        }
        if progress >= 1.0 {
            return (x_range, y_range);
        }

        let offset = self.offset_for_progress(progress);
        let x = ((0.5 + offset).clamp(0.0, 1.0) * x_range).clamp(0.0, x_range);
        let y = (progress * y_range).clamp(0.0, y_range);
        (x, y)
    }

    fn offset_for_progress(&mut self, progress: f64) -> f64 {
        self.ensure_segments(progress);
        self.segments
            .iter()
            .rev()
            .find(|segment| segment.contains(progress))
            .map(|segment| segment.offset_at(progress))
            .unwrap_or(-0.5)
    }

    fn ensure_segments(&mut self, progress: f64) {
        while progress > self.covered_progress && self.covered_progress < 1.0 {
            if self.segments.is_empty() {
                let span = self.random_range(FIRST_BASELINE_SPAN_MIN, FIRST_BASELINE_SPAN_MAX);
                let end = span.min(1.0);
                self.segments
                    .push(WaveSegment::EdgeToBaseline { start: 0.0, end });
                self.covered_progress = end;
                continue;
            }

            let remaining = 1.0 - self.covered_progress;
            if remaining <= END_SETTLE_SPAN_MAX {
                self.segments.push(WaveSegment::BaselineToEdge {
                    start: self.covered_progress,
                    end: 1.0,
                });
                self.covered_progress = 1.0;
                continue;
            }

            let mut span = self.random_range(HALF_WAVE_SPAN_MIN, HALF_WAVE_SPAN_MAX);
            if remaining - span < END_SETTLE_SPAN_MIN {
                span = remaining - END_SETTLE_SPAN_MIN;
            }
            if span <= 0.0 {
                self.segments.push(WaveSegment::BaselineToEdge {
                    start: self.covered_progress,
                    end: 1.0,
                });
                self.covered_progress = 1.0;
                continue;
            }

            let start = self.covered_progress;
            let end = (start + span).min(1.0);
            let amplitude = self.random_range(WAVE_AMPLITUDE_MIN, WAVE_AMPLITUDE_MAX);
            let side = self.next_side;
            self.next_side *= -1.0;
            self.segments.push(WaveSegment::Lobe {
                start,
                end,
                amplitude,
                side,
            });
            self.covered_progress = end;
        }
    }

    fn random_range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.next_random_unit()
    }

    fn next_random_unit(&mut self) -> f64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed as f64 / u64::MAX as f64
    }
}

impl WaveSegment {
    fn contains(&self, progress: f64) -> bool {
        let (start, end) = self.bounds();
        progress >= start && progress <= end
    }

    fn bounds(&self) -> (f64, f64) {
        match *self {
            WaveSegment::EdgeToBaseline { start, end }
            | WaveSegment::BaselineToEdge { start, end }
            | WaveSegment::Lobe { start, end, .. } => (start, end),
        }
    }

    fn offset_at(&self, progress: f64) -> f64 {
        let (start, end) = self.bounds();
        let local = ((progress - start) / (end - start).max(f64::EPSILON)).clamp(0.0, 1.0);
        match *self {
            WaveSegment::EdgeToBaseline { .. } => -0.5 * (local * PI * 0.5).cos(),
            WaveSegment::BaselineToEdge { .. } => 0.5 * (local * PI * 0.5).sin(),
            WaveSegment::Lobe {
                amplitude, side, ..
            } => side * amplitude * (local * PI).sin(),
        }
    }
}

fn custom_material(
    ambient: f64,
    diffuse: f64,
    specular: f64,
    roughness: f64,
    metallic: f64,
    emissive_hex: &str,
    emissive_intensity: f64,
) -> Material {
    let mut params = MaterialParams::default();
    params.ambient = Property::new(ambient);
    params.diffuse = Property::new(diffuse);
    params.specular = Property::new(specular);
    params.roughness = Property::new(roughness);
    params.metallic = Property::new(metallic);
    params.emissive = Property::new(Color::from_hex(emissive_hex));
    params.emissive_intensity = Property::new(emissive_intensity);
    Material::custom(params)
}
