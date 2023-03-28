use pax::api::{ArgsClick, ArgsRender, ArgsScroll, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};


#[pax_app(
    // for i in 0..5 {
    //    <Group transform={Transform2D::align(50%, 50%) * Transform2D::anchor(50%, 50%) * Transform2D::rotate(i * 0.07)} >
            <Text text="Hello world" />
            <Path />
            <Ellipse fill={Color::rgb(0.5,0,1)} width=33.33% height=100% transform={
                Transform2D::align(50%, 0%) * Transform2D::anchor(50%, 0%)
            } />
            <Rectangle fill={Color::rgb(1,0.8,0.1)} width=33.33% height=100% transform={
                Transform2D::align(100%, 0%) * Transform2D::anchor(100%, 0%)
            } />
            <Rectangle fill={Color::rgb(0.25,0.5,0.5)} width=100% height=100% />


    @events {
            Click: [self.handle_click, handle_click_2],
            WillRender: handle_tick,
            Scroll: self.handle_scroll,
            }

    //    </Group>
    // }
)]
pub struct HelloRGB {
    pub rects: Property<Vec<usize>>,
}

impl HelloRGB {
    pub fn handle_click(&mut self, args: ArgsClick) {
        log("click1");
    }
    pub fn handle_click_2(&mut self, args: ArgsClick) {
        log("click2");
    }
    pub fn handle_tick(&mut self, args: ArgsRender) {
        log("tick");
    }
    pub fn handle_scroll(&mut self, args: ArgsScroll) {
        log("scroll");
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

