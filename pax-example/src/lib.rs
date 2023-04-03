use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[pax_app(
    <Group @scroll=self.handle_scroll >
        for i in 0..25 {
            <Rectangle fill={Color::hlc(i * 360.0 / 25.0, 50.0, 180.0)} width=500px height=500px transform={
                Transform2D::anchor(50%, 50%)
                * Transform2D::align(50%, 50%)
                * Transform2D::rotate((i + 2) * rotation)
                * Transform2D::scale(0.75 + (i * rotation), 0.75 + (i * rotation))
            } />
        }
    </Group>

    // Hide hack
    <Group transform={Transform2D::translate(5000.0,5000.0)} >
        <Ellipse />
        <Text />
        <Path />
        <Rectangle />
    </Group>
)]
pub struct HelloRGB {
    pub rotation: Property<f64>,
}

impl HelloRGB {
    pub fn handle_click(&mut self, args: ArgsClick) {
        log("click-ellipse");
    }
    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        const ROTATION_COEFFICIENT: f64 = 0.00010;
        let old_t = self.rotation.get();
        let new_t = old_t + args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(new_t);
    }
    pub fn handle_global_click(&mut self, args: ArgsClick) {
        log("click-anywhere");
    }
    pub fn handle_global_scroll(&mut self, args: ArgsScroll) {
        log("scroll-anywhere");
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

