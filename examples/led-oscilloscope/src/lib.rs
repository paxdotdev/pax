use pax_kit::*;
// The primitive factory uses this package-qualified path in standalone and consuming apps.
extern crate self as led_oscilloscope;
pub mod display;
mod font;
pub mod foreground;
mod geometry;
pub mod ink;
pub mod knob;
pub mod led_glow;
mod message_editor;
pub mod signature_pad;
use display::{frequency_multiplier, Cell, Display, MAX_COLUMNS, ROWS};
pub use foreground::ForegroundInk;
use geometry::disc;
pub use ink::InkPoint;
pub use knob::InstrumentKnob;
pub use led_glow::LedGlow;
pub use message_editor::MessageEditor;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("example.pax")]
pub struct Example {}

/// A responsive instrument cabinet with a live discrete RGB marquee.
/// `message` is uppercased; supported glyphs are documented in the example README.
#[pax]
#[file("lib.pax")]
#[has_helpers]
pub struct LedOscilloscope {
    pub message: Property<String>,
    pub editing_message: Property<bool>,
    pub drawing_mode: Property<bool>,
    pub drawing: Property<Vec<InkPoint>>,
    pub drawing_columns: Property<Vec<u32>>,
    /// Host-controlled suspension, independent of viewport visibility.
    pub paused: Property<bool>,
    pub cyan_amplitude: Property<f64>,
    pub magenta_amplitude: Property<f64>,
    pub yellow_amplitude: Property<f64>,
    pub cyan_frequency: Property<f64>,
    pub magenta_frequency: Property<f64>,
    pub yellow_frequency: Property<f64>,
    pub rate: Property<f64>,
    pub saturation: Property<f64>,
    pub foreground_scroll: Property<usize>,
    pub foreground_period: Property<usize>,
    pub compact: Property<bool>,
    pub margin: Property<f64>,
    pub display_width: Property<f64>,
    pub display_height: Property<f64>,
    pub matrix_width: Property<f64>,
    pub matrix_height: Property<f64>,
    pub matrix_x: Property<f64>,
    pub matrix_y: Property<f64>,
    pub lamp_pitch: Property<f64>,
    pub control_x: Property<f64>,
    pub control_y: Property<f64>,
    pub control_width: Property<f64>,
    pub control_height: Property<f64>,
    pub knob_diameter: Property<f64>,
    pub off_lamps: Property<Vec<PathElement>>,
    pub lens_highlights: Property<Vec<PathElement>>,
    pub cyan: Property<Vec<PathElement>>,
    pub magenta: Property<Vec<PathElement>>,
    pub yellow: Property<Vec<PathElement>>,
    pub lettering: Property<Vec<PathElement>>,
    pub lit_highlights: Property<Vec<PathElement>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Layout {
    width: f64,
    height: f64,
    margin: f64,
    display_width: f64,
    display_height: f64,
    matrix_width: f64,
    matrix_height: f64,
    matrix_x: f64,
    matrix_y: f64,
    control_x: f64,
    control_y: f64,
    control_width: f64,
    control_height: f64,
    diameter: f64,
    columns: usize,
    compact: bool,
}
impl Layout {
    fn new(width: f64, available_height: Option<f64>) -> Self {
        let width = if width.is_finite() { width.max(1.) } else { 1. };
        let available_height = available_height
            .filter(|h| h.is_finite())
            .map(|h| h.max(1.));
        let compact =
            width < 1040. && !available_height.is_some_and(|h| width >= 800. && width >= h * 1.5);
        let margin = (width * 0.018)
            .clamp(10., 24.)
            .min(width * 0.04)
            .min(available_height.unwrap_or(f64::MAX) * 0.04);
        let inset = (width * 0.022).clamp(14., 28.);
        let control_width = if compact {
            width - 2. * margin
        } else {
            (width * 0.36).clamp(320., 480.)
        };
        let display_width = if compact {
            width - 2. * margin
        } else {
            width - 3. * margin - control_width
        };
        let preferred_columns = if width < 600. { 28 } else { 56 };
        let preferred_display_height =
            (display_width - 2. * inset).max(1.) / preferred_columns as f64 * ROWS as f64
                + 2. * inset;
        let preferred_diameter = ((control_width - 12.) / 4. * 0.64).clamp(32., 70.);
        let preferred_control_height = 2. * (preferred_diameter + 70.);
        let height = available_height.unwrap_or_else(|| {
            if compact {
                3. * margin + preferred_display_height + preferred_control_height
            } else {
                2. * margin + preferred_display_height
            }
        });
        let control_height = if compact {
            preferred_control_height.min(height * 0.48)
        } else {
            height - 2. * margin
        };
        let diameter = preferred_diameter.min((control_height / 2. - 70.).max(8.));
        let display_height = if compact {
            (height - 3. * margin - control_height).max(1.)
        } else {
            height - 2. * margin
        };
        let inset = inset.min(display_width * 0.25).min(display_height * 0.25);
        let available_matrix_width = (display_width - 2. * inset).max(1.);
        let available_matrix_height = (display_height - 2. * inset).max(1.);
        // Keep the same 29-row font and square LED pitch at any host aspect ratio.
        // Only the number of visible marquee columns changes; the buffer stays bounded.
        let columns = (available_matrix_width * ROWS as f64 / available_matrix_height)
            .round()
            .clamp(1., MAX_COLUMNS as f64) as usize;
        let pitch =
            (available_matrix_width / columns as f64).min(available_matrix_height / ROWS as f64);
        let matrix_width = pitch * columns as f64;
        let matrix_height = pitch * ROWS as f64;
        let control_x = if compact {
            margin
        } else {
            2. * margin + display_width
        };
        let control_y = if compact {
            2. * margin + display_height
        } else {
            margin
        };
        Self {
            width,
            height,
            margin,
            display_width,
            display_height,
            matrix_width,
            matrix_height,
            matrix_x: (display_width - matrix_width) / 2.,
            matrix_y: (display_height - matrix_height) / 2.,
            control_x,
            control_y,
            control_width,
            control_height,
            diameter,
            columns,
            compact,
        }
    }
}
#[derive(Default)]
struct ScopeState {
    layout: Layout,
    display: Display,
    message: String,
    drawing_mode: bool,
    drawing_columns: Vec<u32>,
    phase: f64,
    scroll: f64,
    last_time: f64,
    last_draw: f64,
    visible: bool,
    previous: Vec<Cell>,
    amplitudes: [f64; 3],
    frequencies: [f64; 3],
}
impl Store for ScopeState {}

#[helpers]
impl LedOscilloscope {
    /// Suggested panel height when the host has a width but no fixed height.
    /// The cabinet itself always fills both dimensions assigned by its host.
    pub fn height(width: f64) -> f64 {
        Layout::new(width, None).height
    }

    pub fn frequency_label(value: f64) -> String {
        format!("{:.2}×", frequency_multiplier(value))
    }
}

impl LedOscilloscope {
    pub fn edit_message(&mut self, _: &NodeContext, _: Event<Click>) {
        self.editing_message.set(true);
    }
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.cyan_amplitude.set(0.82);
        self.magenta_amplitude.set(0.64);
        self.yellow_amplitude.set(0.9);
        self.cyan_frequency.set(0.5);
        self.magenta_frequency.set(0.5);
        self.yellow_frequency.set(0.5);
        self.rate.set(0.45);
        self.saturation.set(0.);
        ctx.provide_store(ScopeState {
            visible: true,
            ..Default::default()
        })
        .expect("oscilloscope mounts once");
    }
    pub fn enter(&mut self, ctx: &NodeContext, _: Event<ViewportProximityEnter>) {
        let _ = ctx.with_store(|s: &mut ScopeState| s.visible = true);
    }
    pub fn exit(&mut self, ctx: &NodeContext, _: Event<ViewportProximityExit>) {
        let _ = ctx.with_store(|s: &mut ScopeState| s.visible = false);
    }
    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let layout = Layout::new(width, Some(height));
        let now = ctx.elapsed_time_millis() as f64;
        let _ = ctx.with_store(|s: &mut ScopeState| {
            let dt = ((now - s.last_time) / 1000.).clamp(0., 0.1);
            s.last_time = now;
            let resized = s.layout != layout;
            if resized {
                s.layout = layout;
                s.display.resize(layout.columns);
                self.publish_layout(layout);
                let mut off = Vec::new();
                let mut highlights = Vec::new();
                let pitch = layout.matrix_width / layout.columns as f64;
                for y in 0..ROWS {
                    for x in 0..layout.columns {
                        let cx = (x as f64 + 0.5) * pitch;
                        let cy = (y as f64 + 0.5) * pitch;
                        disc(&mut off, cx, cy, pitch * 0.31);
                        disc(
                            &mut highlights,
                            cx - pitch * 0.08,
                            cy - pitch * 0.10,
                            pitch * 0.09,
                        );
                    }
                }
                self.off_lamps.set(off);
                self.lens_highlights.set(highlights);
                s.previous.clear();
            }
            let message = self.message.get();
            let drawing_mode = self.drawing_mode.get();
            let drawing_columns = self.drawing_columns.get();
            let changed = message != s.message
                || drawing_mode != s.drawing_mode
                || drawing_columns != s.drawing_columns;
            if changed {
                if drawing_mode {
                    s.display.set_drawing(&drawing_columns);
                } else {
                    s.display.set_text(&message);
                }
                s.message = message;
                s.drawing_mode = drawing_mode;
                s.drawing_columns = drawing_columns;
                s.scroll = 0.;
            }
            let amps = [
                self.cyan_amplitude.get(),
                self.magenta_amplitude.get(),
                self.yellow_amplitude.get(),
            ];
            let frequencies = [
                self.cyan_frequency.get(),
                self.magenta_frequency.get(),
                self.yellow_frequency.get(),
            ];
            let controls = amps != s.amplitudes || frequencies != s.frequencies;
            s.amplitudes = amps;
            s.frequencies = frequencies;
            if s.visible && !self.paused.get() {
                let speed = 0.15 + self.rate.get() * 1.85;
                s.phase = (s.phase + dt * speed).rem_euclid(std::f64::consts::TAU * 20.);
                s.scroll = (s.scroll + dt * 12. * speed) % s.display.period() as f64;
            }
            if !resized
                && !changed
                && !controls
                && (now - s.last_draw < 1000. / 30. || !s.visible || self.paused.get())
            {
                return;
            }
            s.last_draw = now;
            let scroll = s.scroll.floor() as usize;
            s.display.compose(s.phase, scroll, amps, frequencies);
            // Identical occupied cells can still correspond to different source columns.
            self.foreground_scroll.set_if_neq(scroll);
            self.foreground_period.set_if_neq(s.display.period());
            if s.display.cells == s.previous {
                return;
            }
            s.previous.clone_from(&s.display.cells);
            let mut paths: [Vec<PathElement>; 4] = Default::default();
            let mut highlights = Vec::new();
            let pitch = layout.matrix_width / layout.columns as f64;
            for (i, cell) in s.display.cells.iter().enumerate() {
                let index = match cell {
                    Cell::Off => continue,
                    Cell::Cyan => 0,
                    Cell::Magenta => 1,
                    Cell::Yellow => 2,
                    Cell::Text => 3,
                };
                let x = (i % layout.columns) as f64 * pitch + pitch * 0.5;
                let y = (i / layout.columns) as f64 * pitch + pitch * 0.5;
                disc(&mut paths[index], x, y, pitch * 0.31);
                disc(
                    &mut highlights,
                    x - pitch * 0.07,
                    y - pitch * 0.10,
                    pitch * 0.12,
                );
            }
            let [c, m, y, t] = paths;
            self.cyan.set(c);
            self.magenta.set(m);
            self.yellow.set(y);
            self.lettering.set(t);
            self.lit_highlights.set(highlights);
        });
    }
    fn publish_layout(&mut self, l: Layout) {
        self.compact.set(l.compact);
        self.margin.set(l.margin);
        self.display_width.set(l.display_width);
        self.display_height.set(l.display_height);
        self.matrix_width.set(l.matrix_width);
        self.matrix_height.set(l.matrix_height);
        self.matrix_x.set(l.matrix_x);
        self.matrix_y.set(l.matrix_y);
        self.lamp_pitch.set(l.matrix_width / l.columns as f64);
        self.control_x.set(l.control_x);
        self.control_y.set(l.control_y);
        self.control_width.set(l.control_width);
        self.control_height.set(l.control_height);
        self.knob_diameter.set(l.diameter);
    }
}
