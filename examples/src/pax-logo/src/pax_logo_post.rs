use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("pax_logo_post.pax")]
pub struct PaxLogoPost {
    pub fill: Property<Fill>,
}

impl Default for PaxLogoPost {
    fn default() -> Self {
        Self {
            fill: Property::new(Fill::Solid(Color::BLACK)),
        }
    }
}
