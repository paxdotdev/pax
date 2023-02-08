use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Group};
use pax_std::components::{Stacker};

#[pax_app(
    <Group>
        <Text text="Hello" />
        <Rectangle fill={Color::rgb(1, 0, 0)} width=50px height=50px
            transform={Transform2D::rotate(1.25) * Transform2D::translate(50,50)} />
        <Rectangle fill={Color::rgb(1, 0, 0)} width=150px height=150px
            transform={Transform2D::rotate(1.75) * Transform2D::translate(150,150)} />
        <Rectangle fill={Color::rgb(0, 1, 0)} width=150px height=150px
            transform={ Transform2D::rotate(2.25) * Transform2D::translate(300, 100)} />
        <Rectangle fill={Color::rgb(0, 0, 1)} width=300px height=75px
            transform={Transform2D::rotate(3.25) * Transform2D::translate(500, 550)} />
        <Rectangle fill={Color::rgb(0, 0, 0)} width=100% height=100% />
    </Group>
)]
pub struct HelloRGB {}

