#![allow(dead_code)]
use crate::PaxButton;
use crate::PaxText;
use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
    if self.open {
        <Group x=50% y=50% width=300px height=170px>
            <PaxButton width=80px height=30px anchor_y=100% anchor_x=100% y={100%-20px} x={100% -80px -20px -20px} label="Yes" @button_click=handle_yes/>
            <PaxButton width=80px height=30px anchor_y=100% anchor_x=100% y={100% -20px} x={100% -20px} label="No" @button_click=handle_no/>
            <PaxText x=20px y=20px height=15px width=100px text=text/>
            <Rectangle fill=rgb(20, 20, 20)
            corner_radii={
                RectangleCornerRadii::radii(15.0,15.0,15.0,15.0)
            }/>
        </Group>
        <Rectangle fill=rgba(0, 0, 0, 70)/>
    }
)]
#[custom(Default)]
pub struct ConfirmationDialog {
    pub text: Property<String>,
    pub open: Property<bool>,
    pub signal: Property<bool>,
}

impl Default for ConfirmationDialog {
    fn default() -> Self {
        Self {
            text: Property::new("Are you sure?".to_owned()),
            open: Property::default(),
            signal: Property::default(),
        }
    }
}

impl ConfirmationDialog {
    pub fn handle_yes(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.open.set(false);
        self.signal.set(true);
    }

    pub fn handle_no(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.open.set(false);
    }
}
