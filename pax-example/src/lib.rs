use pax::*;
use pax_std::{Spread, Text, Rectangle};

#[pax(
    <Spread cell_count=10 >
        <Rectangle fill=Color::rgba(100%, 100%, 0, 100%) />
        @for i in 0..8 {
            <Text>@{"Index: " + i}</Text>
            <Rectangle fill=Color::rgba(100%, 0, 100%, 100%) />
        }
        <Rectangle transform=@{rotate(self.current_rotation)} fill=Color::rgba(0, 100%, 100%, 100%) />
    </Spread>
)]
pub struct Root {
    pub num_clicks : Property<isize>,
    pub current_rotation: Property<f64>,
}

impl Root {

    #[pax_on(PreRender)] //or long-hand: #[pax_on(Lifecycle::PreRender)]
    pub fn handle_pre_render(&mut self, args: ArgsTick) {
        pax::log(&format!("pre_render from frame {}", args.frame));
        self.current_rotation.set(self.current_rotation.get() + 0.10)
    }

    pub fn some_click_handler(&mut self, args: ArgsClick, i: usize) {

    }
}
