use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[pax_app(
    <Group @scroll=self.handle_scroll >
        for i in 0..25 {
            <Rectangle fill={Color::hlc(ticks + i * 360.0 / 12.5, 75.0, 150.0)} width=300px height=300px transform={
                Transform2D::anchor(50%, 50%)
                * Transform2D::align(50%, 50%)
                * Transform2D::rotate((i + 2) * rotation)
                * Transform2D::scale(1.0 + (i * rotation / 2.0), 1.0 + (i * rotation / 2.0))
                // * Transform2D::scale(1.0 + heartbeat + (i * rotation / 2.0), 1.0 + heartbeat + (i * rotation / 2.0))
            } />
        }
    </Group>

    @events {
        will_render: [self.handle_will_render]
    }
)]
pub struct HelloRGB {
    pub rotation: Property<f64>,
    pub ticks: Property<usize>,
    pub heartbeat: Property<f64>,

}

const ROTATION_COEFFICIENT: f64 = 0.00010;
const HEARTBEAT_AMPLITUDE: f64 = 1.15;

impl HelloRGB {
    pub fn handle_will_render(&mut self, args: ArgsRender) {
        self.ticks.set(args.frames_elapsed);
        if args.frames_elapsed % 400 == 0 {
            self.heartbeat.ease_to(HEARTBEAT_AMPLITUDE, 100, EasingCurve::OutBack);
            self.heartbeat.ease_to_later(-HEARTBEAT_AMPLITUDE / 2.0, 50, EasingCurve::OutBack);
            self.heartbeat.ease_to_later(0.0, 50, EasingCurve::OutBack);
        }
    }
    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        let old_t = self.rotation.get();
        let new_t = f64::max(0.0, old_t - args.delta_y * ROTATION_COEFFICIENT);
        self.rotation.set(new_t);
    }
}

#[pax_type]
#[derive(Default)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

