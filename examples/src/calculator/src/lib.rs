use pax_kit::math::Point2;
use pax_kit::*;
pub mod expression;
pub mod graph;
pub mod key;
pub mod layout;
pub mod model;
pub use key::CalculatorKey;
use layout::{Layout, TextGrid};
use model::Model;

pub struct CalculatorStore {
    model: Model,
    layout: Layout,
    graph_cache: Option<(f64, f64)>,
    drag: Option<(f64, f64, f64, f64)>,
}
impl Store for CalculatorStore {}

#[pax]
#[custom(Defaults)]
pub struct DisplayRow {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub subdued: bool,
}
#[pax]
#[custom(Defaults)]
pub struct GraphLabel {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub text: String,
}
#[pax]
#[custom(Defaults)]
pub struct KeyData {
    pub id: usize,
    pub label: String,
    pub action: String,
    pub tone: usize,
    pub column: f64,
    pub row: f64,
}

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub body_width: Property<f64>,
    pub body_height: Property<f64>,
    pub body_top: Property<f64>,
    pub page_height: Property<f64>,
    pub screen_width: Property<f64>,
    pub screen_height: Property<f64>,
    pub graph_width: Property<f64>,
    pub graph_height: Property<f64>,
    pub graph_scale: Property<f64>,
    pub graph_extent: Property<f64>,
    pub key_width: Property<f64>,
    pub dpad_width: Property<f64>,
    pub graph_mode: Property<bool>,
    pub graph_editing: Property<bool>,
    pub grid_label: Property<String>,
    pub rows: Property<Vec<DisplayRow>>,
    pub input_rows: Property<Vec<DisplayRow>>,
    pub keys: Property<Vec<KeyData>>,
    pub utility_keys: Property<Vec<KeyData>>,
    pub lcd_font_size: Property<f64>,
    pub cell_width: Property<f64>,
    pub line_height: Property<f64>,
    pub text_columns: Property<usize>,
    pub history_height: Property<f64>,
    pub history_extent: Property<f64>,
    pub history_scroll: Property<f64>,
    pub input_y: Property<f64>,
    pub input_height: Property<f64>,
    pub input_start: Property<usize>,
    pub cursor_x: Property<f64>,
    pub cursor_y: Property<f64>,
    pub cursor_visible: Property<bool>,
    pub status: Property<String>,
    pub error: Property<bool>,
    pub graph_status: Property<String>,
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
    pub graph_origin_x: Property<f64>,
    pub graph_origin_y: Property<f64>,
    pub curve: Property<Vec<PathElement>>,
    pub grid: Property<Vec<PathElement>>,
    pub axes: Property<Vec<PathElement>>,
    pub graph_labels: Property<Vec<GraphLabel>>,
    pub graph_range: Property<String>,
    pub silver: Property<Material>,
    pub unlit: Property<Material>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub mono_font: Property<Font>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.unlit.set(Material::unlit());
        self.silver.set(Material::Lit(MaterialParams {
            ambient: Property::new(0.88),
            diffuse: Property::new(0.82),
            specular: Property::new(0.18),
            roughness: Property::new(0.8),
            metallic: Property::new(0.08),
            ..Default::default()
        }));
        let family = if matches!(ctx.platform, Platform::Web) {
            "Courier New, Courier, monospace"
        } else {
            "Courier"
        };
        self.mono_font.set(Font::Web(
            family.into(),
            String::new(),
            FontStyle::Normal,
            FontWeight::Normal,
        ));
        self.keys.set(key_data());
        self.utility_keys.set(utility_data());
        ctx.push_local_store(CalculatorStore {
            model: Model::default(),
            layout: Layout::default(),
            graph_cache: None,
            drag: None,
        });
        self.frame(ctx);
    }
    pub fn frame(&mut self, ctx: &NodeContext) {
        let (w, h) = ctx.bounds_self.get();
        let next = Layout::for_window(w, h);
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| {
            let scale = graph::zoom_scale(store.model.zoom_level);
            let old = store.layout;
            let resized = old != next;
            let zoomed = self.graph_scale.get() != scale;
            if resized || zoomed {
                let view = graph::View {
                    left: self.scroll_x.get(),
                    top: self.scroll_y.get(),
                    width: old.plot_width,
                    height: old.plot_height,
                    scale: self.graph_scale.get().max(graph::DEFAULT_SCALE / 4.),
                }
                .reframe(next.plot_width, next.plot_height, scale);
                self.graph_scale.set(scale);
                self.graph_extent.set(view.extent());
                if old.width > 0. {
                    self.set_scroll(view.left, view.top, next);
                }
                store.drag = None;
                store.model.dirty = true;
                store.model.plot_dirty = true;
            }
            if resized {
                store.layout = next;
                self.body_width.set(next.width);
                self.body_height.set(next.height);
                self.body_top.set(next.top);
                self.page_height.set(next.page_height);
                self.screen_width.set(next.screen_width);
                self.screen_height.set(next.screen_height);
                self.graph_width.set(next.plot_width);
                self.graph_height.set(next.plot_height);
                self.key_width.set((next.width - 44. - 32.) / 5.);
                self.dpad_width
                    .set((2. * self.key_width.get() + 8.).min(140.));
                self.light_x.set(next.width * 0.22);
                self.light_y.set(next.height * 0.15);
            }
            if store.model.home_requested {
                self.set_scroll(
                    self.graph_extent.get() * 0.5 - next.plot_width * 0.5,
                    self.graph_extent.get() * 0.5 - next.plot_height * 0.5,
                    next,
                );
                store.model.home_requested = false;
                store.model.plot_dirty = true;
            }
            if store.model.pan != (0., 0.) {
                self.set_scroll(
                    self.scroll_x.get() + store.model.pan.0,
                    self.scroll_y.get() + store.model.pan.1,
                    next,
                );
                store.model.pan = (0., 0.);
            }
            if store.model.dirty {
                self.refresh_display(&mut store.model, next);
                store.model.dirty = false;
            }
            if store.model.graph_mode {
                let pos = (self.scroll_x.get(), self.scroll_y.get());
                if store.model.plot_dirty
                    || store.graph_cache.map_or(true, |(x, y)| {
                        (pos.0 - x).abs() > 28. || (pos.1 - y).abs() > 28.
                    })
                {
                    self.refresh_plot(&store.model, next);
                    store.graph_cache = Some(pos);
                    store.model.plot_dirty = false;
                }
                let v = graph::View {
                    left: pos.0,
                    top: pos.1,
                    width: next.plot_width,
                    height: next.plot_height,
                    scale,
                };
                self.graph_range.set_if_neq(format!(
                    "x {:.1} … {:.1}   y {:.1} … {:.1}",
                    v.world_x(0.),
                    v.world_x(v.width),
                    v.world_y(v.height),
                    v.world_y(0.)
                ));
            }
        });
        let blink = (ctx.elapsed_millis.get() / 550) % 2 == 0;
        self.cursor_visible
            .set_if_neq(blink && (!self.graph_mode.get() || self.graph_editing.get()));
    }
    fn set_scroll(&mut self, x: f64, y: f64, l: Layout) {
        self.scroll_x
            .set(x.clamp(0., (self.graph_extent.get() - l.plot_width).max(0.)));
        self.scroll_y
            .set(y.clamp(0., (self.graph_extent.get() - l.plot_height).max(0.)));
    }

    fn refresh_display(&mut self, m: &mut Model, l: Layout) {
        self.graph_mode.set(m.graph_mode);
        self.graph_editing.set(m.graph_editing);
        let metrics = TextGrid::new(
            // Keep the shared Calculate character grid clear of overlay scrollbars.
            l.screen_width - if m.graph_mode { 0. } else { 12. },
            l.screen_height,
            if m.graph_mode { 0 } else { m.text_zoom_level },
        );
        self.lcd_font_size.set(metrics.font);
        self.cell_width.set(metrics.cell);
        self.line_height.set(metrics.line);
        self.text_columns.set(metrics.columns);
        self.grid_label.set(if m.graph_mode {
            format!(
                "{}× · {} × {}",
                graph::zoom_scale(m.zoom_level) / graph::DEFAULT_SCALE,
                l.columns,
                l.rows
            )
        } else {
            format!("{} × {}", metrics.columns, metrics.rows)
        });
        let b = m.buffer();
        let chars: Vec<_> = b.text.chars().collect();
        let columns = metrics.columns;
        let input_lines = if m.graph_mode {
            2
        } else {
            metrics.input_lines(chars.len(), l.screen_height)
        };
        let cursor_line = b.cursor / columns;
        let start_line = cursor_line.saturating_sub(input_lines - 1);
        let input_y = if m.graph_mode {
            29.
        } else {
            l.screen_height - 28. - input_lines as f64 * metrics.line
        };
        self.input_y.set(input_y);
        self.input_height.set(input_lines as f64 * metrics.line);
        self.input_start.set(start_line * columns);
        let mut input = Vec::new();
        for row in 0..input_lines {
            let start = (start_line + row) * columns;
            let text = if start < chars.len() {
                chars[start..(start + columns).min(chars.len())]
                    .iter()
                    .collect::<String>()
            } else {
                String::new()
            };
            input.push(DisplayRow {
                id: row,
                x: 0.,
                y: row as f64 * metrics.line,
                text: format!(
                    "{}{}",
                    if row == 0 {
                        if m.graph_mode {
                            "y="
                        } else {
                            "› "
                        }
                    } else {
                        "  "
                    },
                    text
                ),
                subdued: false,
            });
        }
        self.input_rows.set(input);
        self.cursor_x
            .set(14. + (2 + b.cursor % columns) as f64 * metrics.cell);
        self.cursor_y
            .set(input_y + (cursor_line - start_line) as f64 * metrics.line + 2.);
        self.error.set(b.error.is_some());
        let status = if let Some(error) = &b.error {
            format!("{} · col {}", error.message, error.start + 1)
        } else if m.graph_mode {
            if m.graph_editing {
                "ENTER TO PLOT · ESC TO PAN".into()
            } else {
                "DRAG OR SCROLL TO EXPLORE · GRAPH TO CENTER".into()
            }
        } else {
            "RAD · SCROLL HISTORY · ↑ ↓ RECALL".into()
        };
        self.status.set(status);
        if !m.graph_mode {
            let history = m.history_rows(columns);
            let old_max = (self.history_extent.get() - self.history_height.get()).max(0.);
            let old_scroll = self.history_scroll.get();
            let at_bottom = old_max - old_scroll <= 2.;
            let height = (input_y - 48.).max(metrics.line);
            let extent = (history.len() as f64 * metrics.line).max(height);
            let new_max = extent - height;
            let scroll = if m.history_follow_requested || (!m.history.is_empty() && at_bottom) {
                new_max
            } else if old_max > 0. {
                old_scroll / old_max * new_max
            } else {
                0.
            };
            self.history_height.set(height);
            self.history_extent.set(extent);
            self.history_scroll.set(scroll.clamp(0., new_max));
            m.history_follow_requested = false;
            self.rows.set(
                history
                    .into_iter()
                    .enumerate()
                    .map(|(i, (text, subdued))| DisplayRow {
                        id: i,
                        x: 0.,
                        y: i as f64 * metrics.line,
                        text,
                        subdued,
                    })
                    .collect(),
            );
        }
    }

    fn refresh_plot(&mut self, m: &Model, l: Layout) {
        let scale = self.graph_scale.get();
        let extent = self.graph_extent.get();
        let left = (self.scroll_x.get() - 64.).max(0.);
        let top = (self.scroll_y.get() - 64.).max(0.);
        let v = graph::View {
            left,
            top,
            width: (l.plot_width + 128.).min(extent - left),
            height: (l.plot_height + 128.).min(extent - top),
            scale,
        };
        self.graph_origin_x.set(left);
        self.graph_origin_y.set(top);
        let plot = graph::plot(&m.plotted, v);
        let mut curve = Vec::new();
        for pair in &plot.segments {
            line(&mut curve, pair[0], pair[1]);
        }
        self.curve.set(curve);
        self.graph_status.set(if plot.limited {
            "DETAIL LIMIT".into()
        } else if plot.segments.is_empty() {
            "NO RESOLVED CURVE".into()
        } else if self.scroll_x.get() <= 0.5
            || self.scroll_y.get() <= 0.5
            || self.scroll_x.get() >= extent - l.plot_width - 0.5
            || self.scroll_y.get() >= extent - l.plot_height - 0.5
        {
            "WORLD EDGE".into()
        } else {
            "".into()
        });
        let mut grid = Vec::new();
        let mut axes = Vec::new();
        let mut labels = Vec::new();
        let step = graph::tick_step(scale);
        let zero_x = extent * 0.5 - left;
        let zero_y = extent * 0.5 - top;
        let mut x = (v.world_x(0.) / step).ceil() * step;
        while x <= v.world_x(v.width) {
            let px = (x + graph::WORLD_LIMIT) * scale - left;
            line(&mut grid, (px, 0.), (px, v.height));
            if x.abs() > 1e-8 {
                let text = expression::format_number(x);
                labels.push(GraphLabel {
                    id: labels.len(),
                    x: px + 4.,
                    y: zero_y.clamp(4., v.height - 21.) + 3.,
                    width: text.len() as f64 * 6. + 1.,
                    text,
                });
            }
            x += step;
        }
        let mut y = (v.world_y(v.height) / step).ceil() * step;
        while y <= v.world_y(0.) {
            let py = (graph::WORLD_LIMIT - y) * scale - top;
            line(&mut grid, (0., py), (v.width, py));
            if y.abs() > 1e-8 {
                let text = expression::format_number(y);
                labels.push(GraphLabel {
                    id: labels.len(),
                    x: zero_x.clamp(4., v.width - 54.) + 5.,
                    y: py - 17.,
                    width: text.len() as f64 * 6. + 1.,
                    text,
                });
            }
            y += step;
        }
        if zero_x >= 0. && zero_x <= v.width {
            line(&mut axes, (zero_x, 0.), (zero_x, v.height));
        }
        if zero_y >= 0. && zero_y <= v.height {
            line(&mut axes, (0., zero_y), (v.width, zero_y));
        }
        self.grid.set(grid);
        self.axes.set(axes);
        self.graph_labels.set(labels);
    }
}

impl Example {
    pub fn key_down(&mut self, ctx: &NodeContext, event: Event<KeyDown>) {
        if event.keyboard.modifiers.iter().any(|m| {
            matches!(
                m,
                ModifierKey::Control | ModifierKey::Command | ModifierKey::Alt
            )
        }) {
            return;
        }
        let key = &event.keyboard.key;
        let action = match key.as_str() {
            "Enter" => "enter",
            "Backspace" => "backspace",
            "Delete" => "delete",
            "Escape" => "escape",
            "ArrowLeft" => "left",
            "ArrowRight" => "right",
            "ArrowUp" => "up",
            "ArrowDown" => "down",
            "Home" => "home",
            "End" => "end",
            "Tab" => "switch",
            "=" => "enter",
            s if s.chars().count() == 1
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || ".+-*/^() π×÷−".contains(c)) =>
            {
                s
            }
            _ => return,
        };
        event.prevent_default();
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| store.model.action(action));
    }
    pub fn input_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let p = ctx.local_point(Point2::new(event.mouse.x, event.mouse.y));
        let (w, h) = ctx.bounds_self.get();
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| {
            let columns = self.text_columns.get();
            let column = ((p.x * w / self.cell_width.get()).round() as usize)
                .saturating_sub(2)
                .min(columns);
            let row = (p.y * h / self.line_height.get()).floor().max(0.) as usize;
            store
                .model
                .place_cursor(self.input_start.get() + row * columns + column);
        });
    }
    pub fn graph_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| {
            store.model.graph_editing = false;
            store.model.dirty = true;
            store.drag = Some((
                event.mouse.x,
                event.mouse.y,
                self.scroll_x.get(),
                self.scroll_y.get(),
            ));
        });
    }
    pub fn graph_touch(&mut self, ctx: &NodeContext, _event: Event<TouchStart>) {
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| {
            store.model.graph_editing = false;
            store.model.dirty = true;
        });
    }
    pub fn mouse_up(&mut self, ctx: &NodeContext, _event: Event<MouseUp>) {
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| store.drag = None);
    }
    pub fn light_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        // Local coordinates include the outer Scroller's presentation offset.
        let p = ctx.local_point(Point2::new(event.mouse.x, event.mouse.y));
        let (w, h) = ctx.bounds_self.get();
        let lx = p.x * w;
        let ly = p.y * h;
        self.light_x
            .ease_to(lx, Duration::Milliseconds(180.into()), EasingCurve::OutQuad);
        self.light_y
            .ease_to(ly, Duration::Milliseconds(180.into()), EasingCurve::OutQuad);
    }
    pub fn mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let _ = ctx.peek_local_store(|store: &mut CalculatorStore| {
            if let Some((x, y, sx, sy)) = store.drag {
                self.set_scroll(sx + x - event.mouse.x, sy + y - event.mouse.y, store.layout);
            }
        });
    }
}

fn line(path: &mut Vec<PathElement>, a: (f64, f64), b: (f64, f64)) {
    path.push(PathElement::Point(
        Size::Pixels(a.0.into()),
        Size::Pixels(a.1.into()),
    ));
    path.push(PathElement::Line);
    path.push(PathElement::Point(
        Size::Pixels(b.0.into()),
        Size::Pixels(b.1.into()),
    ));
}
fn key_data() -> Vec<KeyData> {
    let rows = [
        [
            ("ln", "ln", 1),
            ("log", "log", 1),
            ("(", "(", 1),
            (")", ")", 1),
            ("⌫", "backspace", 1),
        ],
        [
            ("x²", "square", 1),
            ("1/x", "reciprocal", 1),
            ("xʸ", "^", 1),
            ("π", "π", 1),
            ("e", "e", 1),
        ],
        [
            ("7", "7", 0),
            ("8", "8", 0),
            ("9", "9", 0),
            ("÷", "÷", 1),
            ("clear", "clear", 1),
        ],
        [
            ("4", "4", 0),
            ("5", "5", 0),
            ("6", "6", 0),
            ("×", "×", 1),
            ("x", "x", 1),
        ],
        [
            ("1", "1", 0),
            ("2", "2", 0),
            ("3", "3", 0),
            ("−", "−", 1),
            ("del", "delete", 1),
        ],
        [
            ("0", "0", 0),
            (".", ".", 0),
            ("ans", "ans", 1),
            ("+", "+", 1),
            ("enter", "enter", 2),
        ],
    ];
    rows.into_iter()
        .enumerate()
        .flat_map(|(r, row)| {
            row.into_iter()
                .enumerate()
                .map(move |(c, (label, action, tone))| KeyData {
                    id: r * 5 + c,
                    label: label.into(),
                    action: action.into(),
                    tone,
                    column: c as f64,
                    row: r as f64,
                })
        })
        .collect()
}

fn utility_data() -> Vec<KeyData> {
    [
        ("+", "zoom-in"),
        ("−", "zoom-out"),
        ("√", "sqrt"),
        ("sin", "sin"),
        ("cos", "cos"),
        ("tan", "tan"),
        ("sec", "sec"),
        ("csc", "csc"),
        ("atan", "atan"),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (label, action))| KeyData {
        id,
        label: label.into(),
        action: action.into(),
        tone: 1,
        column: (id / 3) as f64,
        row: (id % 3) as f64,
    })
    .collect()
}
