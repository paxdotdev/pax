use pax_kit::math::Point2;
use pax_kit::*;
use pax_website::SiteTheme;

pax_kit::runnable! { root: crate::Study }

#[pax]
#[file("shape.pax")]
pub struct FaceShape {
    pub tab: Property<bool>,
    pub paint: Property<Fill>,
}

#[pax]
#[file("sample.pax")]
pub struct Sample {
    pub ink_override: Property<Option<Color>>,
    pub tab: Property<bool>,
    pub obsidian: Property<bool>,
    pub cloud: Property<bool>,
    pub gas_color: Property<Color>,
    pub density: Property<f64>,
    pub gas_opacity: Property<f64>,
    pub scale_px: Property<f64>,
    pub phase: Property<f64>,
    pub lamp_x: Property<f64>,
    pub lamp_y: Property<f64>,
    pub scroll_y: Property<f64>,
    pub lamp_scroll_y: Property<f64>,
    pub source_open: Property<bool>,
    pub reduced: Property<bool>,
}

impl Sample {
    fn move_light(&self, ctx: &NodeContext, x: f64, y: f64) {
        if self.reduced.get() {
            return;
        }
        let p = ctx.local_point(Point2::new(x, y));
        self.lamp_scroll_y.set_if_neq(self.scroll_y.get());
        self.lamp_x.set_if_neq(p.x.clamp(0., 1.));
        self.lamp_y.set_if_neq(p.y.clamp(0., 1.));
    }
    pub fn mouse(&mut self, ctx: &NodeContext, ev: Event<MouseMove>) {
        self.move_light(ctx, ev.mouse.x, ev.mouse.y);
    }
    pub fn touch(&mut self, ctx: &NodeContext, ev: Event<TouchMove>) {
        if let Some(t) = ev.touches.first() {
            self.move_light(ctx, t.x, t.y);
        }
    }
    pub fn touch_start(&mut self, ctx: &NodeContext, ev: Event<TouchStart>) {
        if let Some(t) = ev.touches.first() {
            self.move_light(ctx, t.x, t.y);
        }
    }
    pub fn source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.source_open.set(true);
    }
}

#[pax]
#[file("column.pax")]
pub struct SampleColumn {
    pub cloud: Property<bool>,
    pub gas_color: Property<Color>,
    pub density: Property<f64>,
    pub gas_opacity: Property<f64>,
    pub scale_px: Property<f64>,
    pub phase: Property<f64>,
    pub lamp_x: Property<f64>,
    pub lamp_y: Property<f64>,
    pub scroll_y: Property<f64>,
    pub lamp_scroll_y: Property<f64>,
    pub source_open: Property<bool>,
    pub reduced: Property<bool>,
    pub light_theme: Property<bool>,
}

#[pax]
#[has_helpers]
#[file("study.pax")]
pub struct Study {
    pub density: Property<f64>,
    pub gas_opacity: Property<f64>,
    pub scale_px: Property<f64>,
    pub speed: Property<f64>,
    pub tint: Property<u32>,
    pub phase: Property<f64>,
    pub light_theme: Property<bool>,
    pub reduced: Property<bool>,
    pub visible: Property<bool>,
    pub sweep: Property<bool>,
    pub sweep_time: Property<f64>,
    pub last_ms: Property<f64>,
    pub lamp_x: Property<f64>,
    pub lamp_y: Property<f64>,
    pub scroll_y: Property<f64>,
    pub lamp_scroll_y: Property<f64>,
    pub source_open: Property<bool>,
    pub sources: Property<Vec<ExampleSource>>,
    pub view: Property<usize>,
}

#[helpers]
impl Study {
    pub fn hero_width(viewport: f64) -> f64 {
        // hero_section.pax reserves 28% for the logo and 6% for its gutter.
        let row = if viewport < 900. {
            viewport - 48.
        } else {
            viewport * 0.88 * 0.66
        };
        ((row - 12.) * 0.5).max(1.)
    }
    pub fn contrast_ink(dark_cloud: bool, density: f64, opacity: f64) -> Option<Color> {
        if !dark_cloud {
            return None;
        }
        // The native label spans bright gas and the dark page as the wisps move.
        if density < 1.0 || opacity < 0.35 {
            Some(SiteTheme::paper(false))
        } else if density < 2.0 && opacity >= 0.7 {
            Some(Color::from_hex("797977"))
        } else {
            None
        }
    }
    pub fn opacity_label(opacity: f64) -> String {
        format!("Gas opacity  ·  {:.0}%", opacity * 100.)
    }
    pub fn gas_color(tint: u32) -> Color {
        Color::from_hex(match tint {
            1 => "E0EFFE",
            2 => "FBE5CB",
            3 => "ECCEF3",
            _ => "FFFFFF",
        })
    }
    pub fn finish(white: bool) -> Fill {
        SiteTheme::glass_face(white)
    }
    pub fn ink(light: bool) -> Color {
        SiteTheme::paper(light)
    }
    pub fn muted(light: bool) -> Color {
        SiteTheme::muted(light)
    }
    pub fn ground(light: bool) -> Color {
        SiteTheme::ink(light)
    }
    pub fn line(light: bool) -> Color {
        SiteTheme::raised(light)
    }
    pub fn static_face(tint: Color) -> Fill {
        let rgba = tint.to_rgba_0_1();
        Paint::LinearGradient(LinearGradient {
            start: (Size::Percent(0.into()), Size::Percent(0.into())),
            end: (Size::Percent(36.into()), Size::Percent(100.into())),
            stops: [
                (0., 0.83),
                (0.26, 0.97),
                (0.52, 0.88),
                (0.72, 0.99),
                (1., 0.86),
            ]
            .into_iter()
            .map(|(pos, shade)| GradientStop {
                position: Size::Percent((pos * 100.).into()),
                color: Color::from_rgba_0_1([
                    rgba[0] * shade,
                    rgba[1] * shade,
                    rgba[2] * shade,
                    1.,
                ]),
            })
            .collect(),
        })
        .into()
    }
    pub fn sweep_x(seconds: f64) -> f64 {
        // Hold the key still, then cross each subject and return continuously.
        let u = ((seconds % 8.0 - 2.5) / 5.5).clamp(0., 1.);
        0.16 + 0.68 * (0.5 - 0.5 * (std::f64::consts::TAU * u).cos())
    }
}

impl Study {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let cloud = CloudGlass::default();
        self.density.set(cloud.density.get());
        self.gas_opacity.set(cloud.gas_opacity.get());
        self.scale_px.set(cloud.billow_scale.get());
        self.speed.set(4.);
        self.visible.set(true);
        self.lamp_x.set(0.16);
        self.lamp_y.set(0.42);
        self.sources.set(vec![ExampleSource {
            label: "cloud.wgsl".into(),
            language: "wgsl".into(),
            code: concat!(
                include_str!("../../../../pax-std/src/shaders/scene_rim.wgsl"),
                include_str!("../../../../pax-std/src/shaders/cloud.wgsl")
            )
            .into(),
        }]);
        #[cfg(target_arch = "wasm32")]
        motion_preference::observe(ctx, &self.reduced);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = ctx;
    }
    pub fn reset_clock(&mut self, ctx: &NodeContext) {
        self.last_ms.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn advance(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self.last_ms.get()) * 0.001).clamp(0., 0.1);
        self.last_ms.set(now);
        if self.view.get() != 1 && self.speed.get() > 0. {
            self.phase.set(self.phase.get() + dt * self.speed.get());
        }
        if self.sweep.get() {
            let t = (self.sweep_time.get() + dt) % 8.;
            self.sweep_time.set(t);
            self.lamp_scroll_y.set_if_neq(self.scroll_y.get());
            self.lamp_x.set_if_neq(Self::sweep_x(t));
            self.lamp_y.set_if_neq(0.42);
        }
    }
    pub fn visibility(&mut self, _: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
    }
    pub fn toggle_theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_theme.set(!self.light_theme.get());
    }
    pub fn toggle_sweep(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.sweep_time.set(0.);
        self.sweep.set(!self.sweep.get());
    }
    pub fn compare(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.view.set((self.view.get() + 1) % 3);
    }
}

#[cfg(target_arch = "wasm32")]
mod motion_preference {
    use super::*;
    use wasm_bindgen::{closure::Closure, JsCast};
    struct Preference(web_sys::MediaQueryList, Closure<dyn FnMut()>);
    impl Store for Preference {}
    impl std::ops::Drop for Preference {
        fn drop(&mut self) {
            let _ = self
                .0
                .remove_event_listener_with_callback("change", self.1.as_ref().unchecked_ref());
        }
    }
    pub fn observe(ctx: &NodeContext, reduced: &Property<bool>) {
        let Some(query) = web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
        else {
            return;
        };
        reduced.set(query.matches());
        let state = reduced.local();
        let media = query.clone();
        let changed = Closure::wrap(Box::new(move || {
            state.set_if_neq(media.matches());
        }) as Box<dyn FnMut()>);
        if query
            .add_event_listener_with_callback("change", changed.as_ref().unchecked_ref())
            .is_ok()
        {
            let _ = ctx.provide_store(Preference(query, changed));
        }
    }
}
