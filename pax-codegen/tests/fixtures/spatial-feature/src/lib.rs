use pax_kit::*;

#[pax]
#[inlined(
    <Rectangle width=100px height=100px rotate_y=25deg fill=RED/>
)]
pub struct App {}

pax_kit::runnable! { root: crate::App }
