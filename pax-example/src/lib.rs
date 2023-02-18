use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Frame, Group};
use pax_std::components::{Stacker};

#[pax_app(
    <Text text="Hello world" />
    <Rectangle fill={Color::rgb(1,0,1)} width=50% height=100% />
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
