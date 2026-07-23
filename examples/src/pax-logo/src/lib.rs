#![allow(unused_imports)]

use pax_kit::*;

pub mod animated_pax_logo;
pub mod animated_pax_logo_banner;
pub mod animated_pax_logo_post;
pub mod pax_logo;
pub mod pax_logo_board;
pub mod pax_logo_post;
pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use pax_logo::*;
pub use pax_logo_board::*;
pub use pax_logo_post::*;

#[pax]
#[main]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub logo_instances: Property<Vec<u64>>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            logo_instances: Property::new(vec![0]),
        }
    }
}

impl Example {
    pub fn replay(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let next_instance = self
            .logo_instances
            .get()
            .first()
            .copied()
            .unwrap_or_default()
            .wrapping_add(1);
        self.logo_instances.set(vec![next_instance]);
    }
}
