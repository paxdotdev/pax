use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Group};
use pax_std::components::{Stacker};

#[pax_app(
    <Group>
        <Rectangle fill={Color::rgb(100%, 0, 0)} size=(50px, 50px)
            transform={Transform2D::translate(0,0) * Transform2D::rotate(1.25)} />
        <Rectangle fill={Color::rgb(0, 100%, 0)} size=(150px, 150px)
            transform={Transform2D::translate(100px, 100px) * Transform2D::rotate(2.25)} />
        <Rectangle fill={Color::rgb(0, 0, 100%)} size=(300px, 75px)
            transform={Transform2D::translate(200px, 250px) * Transform2D::rotate(3.25)} />
    </Group>
)]
pub struct HelloRGB {}

