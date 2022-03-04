use pax::*;
use pax_std::{Group, Rectangle};

pub struct DeeperStruct {
    a: isize,
    b: &'static str,
}

#[pax(=============>

@template {
    <Group>
        <Rectangle id="rect-a" on_tick=@handle_tick  />
        @for i in (0..10) {
            <Rectangle id="repeated_rect" transform=@{
                translate(i * 100.0, i * 100.0)
            }/>
        }
    </Group>
}

@settings {
    #rect-a {
        fill: rgba(255, 0, 0, 1)
        stroke: {
            color: rgba(0, 0, 0, 1)
            width: 5px
        }
        size: [200px,300px]
        transform: @{
            rotate(current_rotation)
        }
    }

    #repeated-rect {
        fill: rgba(0, 255, 0, 1)
        stroke: {
            color: rgba(255,255,0,1)
        }
        size: [300px, 300px]
    }
})]
pub struct Root {
    pub num_clicks : isize,
    pub current_rotation: f64,
    pub deeper_struct: DeeperStruct,
}

impl Root {
    pub fn new() -> Self {
        Self {
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: DeeperStruct {
                a: 100,
                b: "Profundo!",
            }
        }
    }

    pub fn handle_tick(&mut self, args: ArgsTick) {
        pax::log(&format!("pax::log from frame {}", args.frame));
        self.current_rotation.set(self.current_rotation.get() + 0.10)
    }
}
