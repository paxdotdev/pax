use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo_post.pax")]
pub struct PaxLogoPost {
    pub fill: Property<Paint>,
}

impl Default for PaxLogoPost {
    fn default() -> Self {
        Self {
            fill: Property::new(Paint::Solid(Color::BLACK)),
        }
    }
}
