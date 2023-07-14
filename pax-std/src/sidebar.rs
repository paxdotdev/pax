use pax_lang::*;
use pax_lang::api::{Size2D, Size, Property, Transform2D, EasingCurve, ArgsKeyDown};
use pax_lang::api::numeric::Numeric;
use pax_runtime_api::RuntimeContext;
use crate::primitives::{Frame};
use crate::types::{SidebarDirection};

#[derive(Pax)]
#[inlined(
<Frame transform = {Transform2D::anchor(0%,0%)* Transform2D::align(100%, 0%) * Transform2D::translate(position, 0.0)}
width = 500px
height = 100%
>
slot(1)
</Frame >
<Frame>
slot(0)
</Frame>


@events {
    key_down: handle_key_down,
    did_mount: handle_did_mount,
}

)]
pub struct Sidebar {
    pub enabled: Property<bool>,
    pub position: Property<f64>,
}

impl Sidebar {
    pub fn handle_key_down(&mut self, ctx: RuntimeContext, args: ArgsKeyDown) {
        let enabled = *self.enabled.get();
        if (args.keyboard.key == "c".to_string()) && !enabled {
            self.position.ease_to(-500.0, 80, EasingCurve::InQuad);
            self.enabled.set(true);
        }
        if (args.keyboard.key == "c".to_string()) && enabled {
            self.position.ease_to(0.0, 80, EasingCurve::OutBack);
            self.enabled.set(false);
        }
    }

    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        self.enabled.set(false);
        self.position.set(0.0);
    }
}

