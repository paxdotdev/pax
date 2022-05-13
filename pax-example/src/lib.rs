use pax::*;
use pax_std::{Spread, Text, Rectangle};

#[pax_root(
    <Spread cell_count=10 >
        <Rectangle fill=Color::rgba(100%, 100%, 0, 100%) />
        for i in 0..8 {
            <Group>
                <Text>`Index: {i}`</Text>
                <Rectangle fill=Color::rgba(100%, 0, 100%, 100%) />
            </Group>
        }
        <Group transform={rotate(self.current_rotation)}>
            <Text>{JABBERWOCKY}</Text>
            <Rectangle  fill=Color::rgba(0, 100%, 100%, 100%) />
        </Group>
    </Spread>
)]
pub struct HelloWorld {
    pub num_clicks : Property<i64>,
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

const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves
Did gyre and gimble in the wabe:
All mimsy were the borogoves,
And the mome raths outgrabe.

“Beware the Jabberwock, my son!
The jaws that bite, the claws that catch!
Beware the Jubjub bird, and shun
The frumious Bandersnatch!”

He took his vorpal sword in hand;
Long time the manxome foe he sought—
So rested he by the Tumtum tree
And stood awhile in thought.

And, as in uffish thought he stood,
The Jabberwock, with eyes of flame,
Came whiffling through the tulgey wood,
And burbled as it came!

One, two! One, two! And through and through
The vorpal blade went snicker-snack!
He left it dead, and with its head
He went galumphing back.

“And hast thou slain the Jabberwock?
Come to my arms, my beamish boy!
O frabjous day! Callooh! Callay!”
He chortled in his joy.

’Twas brillig, and the slithy toves
Did gyre and gimble in the wabe:
All mimsy were the borogoves,
And the mome raths outgrabe.
"#;

