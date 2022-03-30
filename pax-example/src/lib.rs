use pax::*;
use pax::api::{Property};
use pax_std::{Spread};

#[pax(
    <Spread cell_count=10 >
        @for i in (0..10) {
            <Text>@{"Index: " + i}</Text>
            <Rectangle fill=Color::rgba(100%, 45%, 25%, 100%) />
        }
    </Spread>
)]
pub struct Root {
    pub num_clicks : Property<isize>,
    pub current_rotation: Property<f64>,
}

impl Root {

    #[pax_on(pre_render)]
    pub fn handle_pre_render(&mut self, args: ArgsTick) {
        pax::log(&format!("pax::log from frame {}", args.frame));
        self.current_rotation.set(self.current_rotation.get() + 0.10)
    }

    pub fn some_method(&mut self, args: ArgsClick, i: usize) {

    }
}
