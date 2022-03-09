use pax::*;
use pax_std::{Spread};

#[pax(
    @template {
        <Group>
            <Rectangle>
        </Group>
        <Spread>
            @for i in (0..10) {
                <Rectangle fill=Color::rgba(1.0, 1.0, 1.0, 1.0)/>
            }
        </Group>
    }
)]
pub struct Root {
    pub num_clicks : Box<dyn pax::api::Property<isize>>,
    pub current_rotation: Box<dyn pax::api::Property<f64>>,
}

impl Root {
    pub fn handle_pre_render(&mut self, args: ArgsTick) {
        pax::log(&format!("pax::log from frame {}", args.frame));
        self.current_rotation.set(self.current_rotation.get() + 0.10)
    }
}
