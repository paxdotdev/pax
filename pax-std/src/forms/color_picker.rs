#[allow(unused_imports)] // Components referenced by the Pax template.
use crate::{
    Button, Ellipse, Group, Image, Overlay, Rectangle, Slider, Text, Textbox,
};
use pax_engine::api::*;
use pax_engine::*;
use pax_runtime::api::NodeContext;

/// Edits a color with a saturation/lightness palette, hue and opacity sliders,
/// and four numeric RGBA fields. Bind `color` to share edits with the parent.
#[pax]
#[engine_import_path("pax_engine")]
#[file("forms/color_picker.pax")]
pub struct ColorPicker {
    /// Selected color. External changes update all controls. Defaults to SLATE.
    pub color: Property<Color>,
    #[doc(hidden)]
    pub _values: Property<ColorPickerValues>,
    #[doc(hidden)]
    pub _checker: Property<Vec<u8>>,
    #[doc(hidden)]
    pub _dragging: Property<bool>,
}

/// A clickable color sample that opens a viewport-anchored `ColorPicker`.
/// A click or tap outside closes the editor and activates the clicked control.
#[pax]
#[engine_import_path("pax_engine")]
#[file("forms/color_picker_swatch.pax")]
pub struct ColorPickerSwatch {
    /// Selected color shared by the swatch and its editor. Defaults to SLATE.
    pub color: Property<Color>,
    #[doc(hidden)]
    pub _open: Property<bool>,
    #[doc(hidden)]
    pub _right: Property<f64>,
    #[doc(hidden)]
    pub _top: Property<f64>,
    #[doc(hidden)]
    pub _bottom: Property<f64>,
}

impl ColorPickerSwatch {
    pub fn toggle(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self._open.set(!self._open.get());
    }

    pub fn dismiss(&mut self, _: &NodeContext, _: Event<Click>) {
        self._open.set(false);
    }

    pub fn key_down(&mut self, _: &NodeContext, event: Event<KeyDown>) {
        if self._open.get() && event.keyboard.key == "Escape" {
            self._open.set(false);
        }
    }

    pub fn position(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        let bounds = event.current.bounds;
        self._right.set_if_neq(bounds.x1);
        self._top.set_if_neq(bounds.y0);
        self._bottom.set_if_neq(bounds.y1);
        if !event.current.is_in_viewport() {
            self._open.set_if_neq(false);
        }
    }
}

#[doc(hidden)]
#[pax]
#[engine_import_path("pax_engine")]
pub struct ColorPickerValues {
    pub hue: f64,
    pub saturation: f64,
    pub lightness: f64,
    pub alpha: f64,
    pub pure_hue: Color,
    pub opaque: Color,
    pub transparent: Color,
}

impl ColorPicker {
    pub fn mount(&mut self, _: &NodeContext) {
        let color = self.color.local();
        let deps = [color.untyped()];
        self._values.replace_with(LocalProperty::computed(
            move || {
                let color = color.get();
                let [hue, saturation, lightness, alpha] = picker_hsla(&color);
                let [r, g, b, _] = color.to_rgba_0_1();
                ColorPickerValues {
                    hue,
                    saturation,
                    lightness,
                    alpha,
                    pure_hue: from_hsla([hue, 1.0, 0.5, 1.0]),
                    opaque: Color::from_rgba_0_1([r, g, b, 1.0]),
                    transparent: Color::from_rgba_0_1([r, g, b, 0.0]),
                }
            },
            &deps,
        ));
        // Two rows of checks scale with the opacity track, without a project asset.
        self._checker.set(
            (0..96)
                .flat_map(|i| {
                    let gray = if (i % 48 + i / 48) % 2 == 0 { 128 } else { 210 };
                    [gray, gray, gray, 255]
                })
                .collect(),
        );
    }

    pub fn hue_change(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        let mut hsla = picker_hsla(&self.color.get());
        hsla[0] = event.value;
        self.color.set_if_neq(from_hsla(hsla));
    }

    pub fn alpha_change(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        let mut hsla = picker_hsla(&self.color.get());
        hsla[3] = event.value;
        self.color.set_if_neq(from_hsla(hsla));
    }

    fn pick(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let point = ctx.local_point(pax_engine::math::Point2::new(x, y));
        let [hue, _, _, alpha] = picker_hsla(&self.color.get());
        self.color.set_if_neq(from_hsla([
            hue,
            point.x.clamp(0.0, 1.0),
            1.0 - point.y.clamp(0.0, 1.0),
            alpha,
        ]));
    }

    pub fn mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        if event.mouse.button == MouseButton::Left {
            self._dragging.set(true);
            self.pick(ctx, event.mouse.x, event.mouse.y);
        }
    }
    pub fn mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        if self._dragging.get() {
            self.pick(ctx, event.mouse.x, event.mouse.y);
        }
    }
    pub fn mouse_up(&mut self, _: &NodeContext, _: Event<MouseUp>) {
        self._dragging.set(false);
    }
    pub fn mouse_cancel(&mut self, _: &NodeContext, _: Event<MouseCancel>) {
        self._dragging.set(false);
    }
    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            event.prevent_default();
            self.pick(ctx, touch.x, touch.y);
        }
    }
    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(touch) = event.touches.first() {
            event.prevent_default();
            self.pick(ctx, touch.x, touch.y);
        }
    }
    pub fn touch_end(&mut self, ctx: &NodeContext, event: Event<TouchEnd>) {
        if let Some(touch) = event.touches.first() {
            self.pick(ctx, touch.x, touch.y);
        }
    }
}

// Each native editor keeps its own draft, including incomplete or invalid input.
// A parent color change replaces the drafts through the same reactive binding.
#[doc(hidden)]
#[pax]
#[engine_import_path("pax_engine")]
#[file("forms/color_picker_channel.pax")]
pub struct ColorPickerChannel {
    pub color: Property<Color>,
    pub index: Property<usize>,
    pub _draft: Property<String>,
    pub _caption: Property<String>,
    pub _valid: Property<bool>,
}

impl ColorPickerChannel {
    pub fn mount(&mut self, _: &NodeContext) {
        let color = self.color.local();
        let index = self.index.local();
        let deps = [color.untyped(), index.untyped()];
        self._draft.replace_with(LocalProperty::computed(
            move || {
                let v = color.get().to_rgba_0_1()[index.get().min(3)];
                ((v * 255.0).round() as u8).to_string()
            },
            &deps,
        ));

        let index = self.index.local();
        let deps = [index.untyped()];
        self._caption.replace_with(LocalProperty::computed(
            move || ["R", "G", "B", "A"][index.get().min(3)].to_owned(),
            &deps,
        ));
        let draft = self._draft.local();
        let deps = [draft.untyped()];
        self._valid.replace_with(LocalProperty::computed(
            move || channel_byte(&draft.get()).is_some(),
            &deps,
        ));
    }

    pub fn commit(&mut self, _: &NodeContext, event: Event<TextboxChange>) {
        self.commit_draft(&event.text);
    }

    pub fn on_pre_unmount(&mut self, _: &NodeContext) {
        // A dismissing tap can remove the editor before the native blur commits.
        self.commit_draft(&self._draft.get());
    }

    fn commit_draft(&self, text: &str) {
        let Some(byte) = channel_byte(text) else {
            return;
        };
        let index = self.index.get().min(3);
        let value = f64::from(byte) / 255.0;
        let current = self.color.get();
        let mut rgba = current.to_rgba_0_1();
        if (rgba[index] * 255.0).round() as u8 == byte {
            self._draft.set_if_neq(byte.to_string());
            return;
        }
        let next = if index == 3 {
            let mut hsla = picker_hsla(&current);
            hsla[3] = value;
            from_hsla(hsla)
        } else {
            rgba[index] = value;
            Color::from_rgba_0_1(rgba)
        };
        self.color.set_if_neq(next);
        self._draft.set_if_neq(byte.to_string());
    }
}

fn channel_byte(text: &str) -> Option<u8> {
    let text = text.trim();
    if text.is_empty() {
        Some(0)
    } else {
        text.parse().ok()
    }
}

fn picker_hsla(color: &Color) -> [f64; 4] {
    // RGB cannot encode the selected hue at zero saturation, or saturation at
    // black/white. Keep those coordinates when the input already carries HSL.
    let (h, s, l, a) = match color {
        Color::hsl(h, s, l) => (h, s, l, 1.0),
        Color::hsla(h, s, l, a) => (h, s, l, a.to_float_0_1()),
        _ => return color.to_hsla_0_1(),
    };
    let h = h.to_float_0_1();
    [
        if h == 1.0 { h } else { h.rem_euclid(1.0) },
        s.to_float_0_1(),
        l.to_float_0_1(),
        a,
    ]
}

fn from_hsla([h, s, l, a]: [f64; 4]) -> Color {
    Color::hsla(
        Rotation::Percent((h * 100.0).into()),
        ColorChannel::Percent((s * 100.0).into()),
        ColorChannel::Percent((l * 100.0).into()),
        ColorChannel::Percent((a * 100.0).into()),
    )
}
