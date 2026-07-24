#![allow(unused_imports)]

use pax_kit::math::Point2;
use pax_kit::*;

const HOVER_INTENSITY: f64 = 2.15;
const TOUCH_INTENSITY: f64 = 2.65;
const HOVER_SCALE: f64 = 1.018;
const PRESSED_SCALE: f64 = 0.982;
const SURFACE_SPECULAR: f64 = 0.56;

#[pax]
#[file("glow_button.pax")]
pub struct GlowButton {
    pub label: Property<String>,
    pub base_color: Property<Color>,
    pub glow_color: Property<Color>,

    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub glow_intensity: Property<f64>,
    pub glow_enabled: Property<bool>,
    pub hovered: Property<bool>,
    pub touch_active: Property<bool>,
    pub active_touch_identifier: Property<i64>,
    pub shell_scale: Property<f64>,
    pub surface_material: Property<Material>,
    pub unlit_material: Property<Material>,
}

impl GlowButton {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        self.light_x.set(width * 0.5);
        self.light_y.set(height * 0.5);
        self.glow_intensity.set(0.0);
        self.glow_enabled.set(false);
        self.hovered.set(false);
        self.touch_active.set(false);
        self.active_touch_identifier.set(-1);
        self.shell_scale.set(1.0);
        self.surface_material.set(resting_surface_material());
        self.unlit_material.set(Material::unlit());
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        if !self.hovered.get()
            && !self.touch_active.get()
            && self.glow_enabled.get()
            && self.glow_intensity.get() <= f64::EPSILON
        {
            self.glow_enabled.set(false);
        }
    }

    pub fn mouse_over(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        if self.touch_active.get() {
            return;
        }
        self.hovered.set(true);
        self.glow_enabled.set(true);
        self.glow_intensity.ease_to(
            HOVER_INTENSITY,
            Duration::Milliseconds(135.into()),
            EasingCurve::OutQuad,
        );
        self.surface_material.ease_to(
            active_surface_material(),
            Duration::Milliseconds(135.into()),
            EasingCurve::OutQuad,
        );
        self.shell_scale.ease_to(
            HOVER_SCALE,
            Duration::Milliseconds(150.into()),
            EasingCurve::OutQuad,
        );
    }

    pub fn mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        if self.hovered.get() && !self.touch_active.get() {
            self.move_light(ctx, event.mouse.x, event.mouse.y, true);
        }
    }

    pub fn mouse_out(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        if self.touch_active.get() {
            return;
        }
        self.hovered.set(false);
        self.glow_intensity.ease_to(
            0.0,
            Duration::Milliseconds(230.into()),
            EasingCurve::OutQuad,
        );
        self.surface_material.ease_to(
            resting_surface_material(),
            Duration::Milliseconds(230.into()),
            EasingCurve::OutQuad,
        );
        self.shell_scale.ease_to(
            1.0,
            Duration::Milliseconds(180.into()),
            EasingCurve::OutQuad,
        );
    }

    pub fn mouse_down(&mut self, _ctx: &NodeContext, _event: Event<MouseDown>) {
        if !self.touch_active.get() {
            self.shell_scale.ease_to(
                PRESSED_SCALE,
                Duration::Milliseconds(70.into()),
                EasingCurve::OutQuad,
            );
        }
    }

    pub fn mouse_up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        if !self.touch_active.get() {
            self.shell_scale.ease_to(
                if self.hovered.get() { HOVER_SCALE } else { 1.0 },
                Duration::Milliseconds(130.into()),
                EasingCurve::OutBack,
            );
        }
    }

    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        let Some(touch) = event.touches.first() else {
            return;
        };

        self.touch_active.set(true);
        self.active_touch_identifier.set(touch.identifier);
        self.move_light(ctx, touch.x, touch.y, false);
        self.glow_enabled.set(true);
        self.glow_intensity.ease_to(
            TOUCH_INTENSITY,
            Duration::Milliseconds(145.into()),
            EasingCurve::OutQuad,
        );
        self.surface_material.ease_to(
            active_surface_material(),
            Duration::Milliseconds(145.into()),
            EasingCurve::OutQuad,
        );
        self.shell_scale.ease_to(
            PRESSED_SCALE,
            Duration::Milliseconds(85.into()),
            EasingCurve::OutQuad,
        );
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if !self.touch_active.get() {
            return;
        }
        let identifier = self.active_touch_identifier.get();
        if let Some(touch) = event
            .touches
            .iter()
            .find(|touch| touch.identifier == identifier)
        {
            self.move_light(ctx, touch.x, touch.y, true);
        }
    }

    pub fn touch_end(&mut self, ctx: &NodeContext, event: Event<TouchEnd>) {
        if !self.touch_active.get() {
            return;
        }
        let identifier = self.active_touch_identifier.get();
        let Some(touch) = event
            .touches
            .iter()
            .find(|touch| touch.identifier == identifier)
        else {
            return;
        };

        self.move_light(ctx, touch.x, touch.y, false);
        self.touch_active.set(false);
        self.active_touch_identifier.set(-1);

        // Queue the fade instead of replacing the glow-in. Even a very quick tap therefore
        // reveals a complete, legible pulse at the point of contact.
        self.glow_intensity.ease_to_later(
            0.0,
            Duration::Milliseconds(260.into()),
            EasingCurve::OutQuad,
        );
        self.surface_material.ease_to_later(
            resting_surface_material(),
            Duration::Milliseconds(260.into()),
            EasingCurve::OutQuad,
        );
        self.shell_scale.ease_to(
            1.0,
            Duration::Milliseconds(190.into()),
            EasingCurve::OutBack,
        );
    }

    pub fn touch_cancel(&mut self, ctx: &NodeContext, event: Event<TouchCancel>) {
        if !self.touch_active.get() {
            return;
        }
        let identifier = self.active_touch_identifier.get();
        if let Some(touch) = event
            .touches
            .iter()
            .find(|touch| touch.identifier == identifier)
        {
            self.move_light(ctx, touch.x, touch.y, false);
        }

        self.touch_active.set(false);
        self.active_touch_identifier.set(-1);
        // The platform aborted this contact rather than releasing it normally. Clear optimistic
        // feedback promptly; ordinary Scroller pans continue through touch_end instead.
        self.glow_intensity.ease_to(
            0.0,
            Duration::Milliseconds(180.into()),
            EasingCurve::OutQuad,
        );
        self.surface_material.ease_to(
            resting_surface_material(),
            Duration::Milliseconds(180.into()),
            EasingCurve::OutQuad,
        );
        self.shell_scale.ease_to(
            1.0,
            Duration::Milliseconds(150.into()),
            EasingCurve::OutBack,
        );
    }

    fn move_light(&mut self, ctx: &NodeContext, x: f64, y: f64, eased: bool) {
        let local = ctx.local_point(Point2::new(x, y));
        let (width, height) = ctx.bounds_self.get();
        let light_x = local.x.clamp(0.0, 1.0) * width;
        let light_y = local.y.clamp(0.0, 1.0) * height;

        if eased {
            self.light_x.ease_to(
                light_x,
                Duration::Milliseconds(52.into()),
                EasingCurve::OutQuad,
            );
            self.light_y.ease_to(
                light_y,
                Duration::Milliseconds(52.into()),
                EasingCurve::OutQuad,
            );
        } else {
            self.light_x.set(light_x);
            self.light_y.set(light_y);
        }
    }
}

fn active_surface_material() -> Material {
    Material::glossy(SURFACE_SPECULAR)
}

fn resting_surface_material() -> Material {
    let Material::Lit(params) = active_surface_material() else {
        unreachable!("glossy materials are always lit");
    };
    // The default scene ambient dims a lit surface to 35%. Raising only the ambient response
    // to its reciprocal makes the zero-direct-light endpoint match identity rendering exactly.
    // The material and light intensity animate together, so disabling the exhausted light is
    // visually continuous.
    params
        .ambient
        .set(1.0 / SceneLighting::DEFAULT_AMBIENT_INTENSITY);
    Material::custom(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resting_material_matches_identity_under_default_ambient() {
        let Material::Lit(params) = resting_surface_material() else {
            panic!("resting surface must remain light-reactive");
        };
        let resolved_brightness = params.ambient.get() * SceneLighting::DEFAULT_AMBIENT_INTENSITY;
        assert!((resolved_brightness - 1.0).abs() < f64::EPSILON);
    }
}
