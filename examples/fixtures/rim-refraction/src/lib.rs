use pax_kit::math::Point2;
use pax_kit::*;
use pax_website::SiteTheme;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("study_button.pax")]
pub struct StudyButton {
    pub label: Property<String>,
    pub url: Property<String>,
    pub github: Property<bool>,
    pub view: Property<usize>,
}

#[pax]
#[file("face.pax")]
pub struct Face {
    pub github: Property<bool>,
}

#[pax]
#[has_helpers]
#[file("lib.pax")]
pub struct Example {
    pub view: Property<usize>,
    pub light: Property<usize>,
    pub subjects: Property<usize>,
    pub playing: Property<bool>,
    pub visible: Property<bool>,
    pub phase: Property<f64>,
    pub last_time: Property<f64>,
    pub input_seen: Property<bool>,
    pub pointer_x: Property<f64>,
    pub pointer_y: Property<f64>,
}

#[helpers]
impl Example {
    // Use the same finish as GlassAction without importing the website's asset collection.
    pub fn face_paint(github: bool) -> Paint {
        if github {
            SiteTheme::obsidian_paint()
        } else {
            SiteTheme::opal_paint()
        }
    }
    pub fn face_material(github: bool) -> Material {
        if github {
            SiteTheme::obsidian()
        } else {
            SiteTheme::opal()
        }
    }
    pub fn label_color(github: bool) -> Color {
        if github {
            SiteTheme::paper()
        } else {
            SiteTheme::ink()
        }
    }
    pub fn strip_percent(phase: f64) -> f64 {
        50. + 45. * (phase * std::f64::consts::TAU).sin()
    }
}

impl Example {
    pub fn view(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.view.set((self.view.get() + 1) % 3);
    }
    pub fn light(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light.set((self.light.get() + 1) % 4);
    }
    pub fn subjects(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.subjects.set((self.subjects.get() + 1) % 3);
    }
    pub fn play(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.playing.set(!self.playing.get());
    }
    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn advance(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = now - self.last_time.get();
        self.last_time.set(now);
        if (0.0..100.).contains(&elapsed) {
            self.phase.set((self.phase.get() + elapsed / 6000.).fract());
        }
    }
    fn point(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let p = ctx.local_point(Point2::new(x, y));
        let (w, h) = ctx.bounds_self.get();
        self.pointer_x.set_if_neq(p.x * w);
        self.pointer_y.set_if_neq(p.y * h);
        self.input_seen.set_if_neq(true);
    }
    pub fn mouse(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        self.point(ctx, event.mouse.x, event.mouse.y);
    }
    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(t) = event.touches.first() {
            self.point(ctx, t.x, t.y);
        }
    }
    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(t) = event.touches.first() {
            self.point(ctx, t.x, t.y);
        }
    }
}

#[pax]
#[file("silhouettes.pax")]
pub struct Silhouettes {
    pub phase: Property<f64>,
}

#[pax]
#[file("button_pair.pax")]
pub struct ButtonPair {
    pub view: Property<usize>,
}
