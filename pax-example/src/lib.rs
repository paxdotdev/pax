use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Frame, Group};
use pax_std::components::{Stacker};

#[pax_app(
    <Group transform={Transform2D::align(50%, 50%) * Transform2D::anchor(50%, 50%) * Transform2D::rotate(0.07)} >
        <Text text="Hello world" />
        <Rectangle fill={Color::rgb(1,0.5,0)} width=33.33% height=100% />
        <Rectangle fill={Color::rgb(0.5,0,1)} width=33.33% height=100% transform={
            Transform2D::align(50%, 0%) * Transform2D::anchor(50%, 0%)
        } />
        <Rectangle fill={Color::rgb(0,1,0.5)} width=33.33% height=100% transform={
            Transform2D::align(100%, 0%) * Transform2D::anchor(100%, 0%)
        } />
    </Group>
)]
pub struct HelloRGB {
    pub rects: Property<Vec<usize>>
}

#[pax_type]
#[derive(Default)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
