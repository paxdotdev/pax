use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Group};
use pax_std::components::{Stacker};

#[pax_app(
    <Group>
        <Rectangle fill={Color::rgb(1, 0, 0)} size=(50px, 50px)
            transform={Transform2D::translate(0,0) * Transform2D::rotate(1.25)} />
        <Rectangle fill={Color::rgb(0, 1, 0)} size=(150px, 150px)
            transform={Transform2D::translate(100, 100) * Transform2D::rotate(2.25)} />
        <Rectangle fill={Color::rgb(0, 0, 1)} size=(300px, 75px)
            transform={Transform2D::translate(200, 250) * Transform2D::rotate(3.25)} />
    </Group>
)]
pub struct HelloRGB {}

