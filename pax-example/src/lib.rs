use pax::api::{ArgsClick, ArgsRender, EasingCurve};
use pax::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Ellipse, Frame, Group, Path, Rectangle, Text};

#[pax_app(
    <Group transform={Transform2D::align(50%, 50%) * Transform2D::anchor(50%, 50%) * Transform2D::rotate(0.07)} >
        <Text text="Hello world" />
        <Path segments={Path::curve_to(Path::line_to(Path::start(), (0.0,0.0), (500.0, 500.0)), (800.0,500.0) , (0.0,0.0) , (700.0,300.0) )}  />
        <Ellipse fill={Color::rgb(0,0.5,0)} width=33.33% height=100% />
        <Rectangle fill={Color::rgb(1,0.5,0)} width=33.33% height=100% />

    </Group>
)]
pub struct HelloRGB {
    pub rects: Property<Vec<usize>>,
}

#[pax_type]
#[derive(Default)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

