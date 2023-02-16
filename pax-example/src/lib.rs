use pax::*;
use pax::api::{EasingCurve, ArgsRender, ArgsClick};
use pax_std::primitives::{Text, Rectangle, Frame};
use pax_std::components::{Stacker};

#[pax_app(
    // for rect in self.rects {
    //    <Rectangle width={rect.width} height={rect.height} transform={Transform2D::translate(rect.x, rect.y)} />
    // }
    <Text text="Hello World" />
    <Rectangle fill={Color::rgb(1,0,1)} />

    /*<Stacker cells=2>
        <Rectangle fill={Color::rgb(1,1,1)} />
        <Frame transform={Transform2D::align(50%, 50%) * Transform2D::anchor(50%, 50%) * Transform2D::rotate(0.08)}>
            <Text text="Hello" fill={Color::rgb(1,1,1)} />
            <Rectangle fill={Color::rgb(1, 0, 0)} width=50px height=50px
                transform={Transform2D::rotate(1.25) * Transform2D::translate(50,50)} />
            <Rectangle fill={Color::rgb(1, 0, 0)} width=150px height=150px
                transform={Transform2D::rotate(1.75) * Transform2D::translate(150,150)} />
            <Rectangle fill={Color::rgb(1, 1, 0)} width=150px height=150px
                transform={Transform2D::rotate(2.25) * Transform2D::translate(300, 100)} />
            <Rectangle fill={Color::rgb(0, 1, 1)} width=300px height=75px
                transform={Transform2D::rotate(3.25) * Transform2D::translate(500, 550)} />
            <Rectangle fill={Color::rgb(0, 0, 0)} width=100% height=100% />
        </Frame>
    </Stacker>*/
)]
pub struct HelloRGB {
    rects: Property<Vec<usize>>
}


#[pax_type]
#[derive(Default)]
pub struct RectDef {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
