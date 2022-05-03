use pax::*;
use pax_std::{Spread, Text, Rectangle};

#[pax(
    <Spread cell_count=10 >
        <Rectangle fill=Color::rgba(100%, 100%, 0, 100%) />
        for i in 0..8 {
            <Group>
                <Text>`_Index_: {i}`</Text>
                <Rectangle fill=Color::rgba(100%, 0, 100%, 100%) />
            </Group>
        }
        <Rectangle transform={rotate(self.current_rotation)} fill=Color::rgba(0, 100%, 100%, 100%) />
    </Spread>
)]
pub struct HelloWorld {
    pub num_clicks : Property<isize>,
    pub current_rotation: Property<f64>,
}

impl HelloWorld {

    #[pax_on(PreRender)] //or long-hand: #[pax_on(Lifecycle::PreRender)]
    pub fn handle_pre_render(&mut self, args: ArgsTick) {
        if args.frames_elapsed % 180 == 0 {
            //every 3s
            pax::log(&format!("pax::log from frame {}", args.frames_elapsed));
            let new_rotation = self.current_rotation.get() + (2.0 * f64::PI());
            self.current_rotation.ease_to(new_rotation, 120, EasingCurve::InOutBack );
            self.current_rotation.ease_to_later(0.0, 1, EasingCurve::Linear );
        }
    }

}
