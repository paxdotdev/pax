#![allow(dead_code)]

use crate::*;
use pax_engine::api::*;
use pax_engine::*;
use std::cmp::Ordering;
use std::iter;

const IN_OUT_TIME: u64 = 10;

#[pax]
#[inlined(
    if self.shown {
        <Group x=100% y={y_pos} width=400px height={height}>
            <Group x=50% y=50% width={100%-20px} height={100%-20px}>
                <Text x=40px height=100% width=70% text=message align={TextAlignHorizontal::Left}/>
                <Button width=70px height=30px x=90% y=50% label="Undo" @button_click=handle_trigger/>
                <Rectangle fill=rgb(20, 20, 20)
                corner_radii={
                    RectangleCornerRadii::radii(15.0,15.0,15.0,15.0)
                }/>
            </Group>
        </Group>
    }

    @settings {
        @mount: on_mount
        @pre_render: pre_render
    }
)]
pub struct Toast {
    pub shown: Property<bool>,
    pub message: Property<String>,
    pub height: Property<Size>,
    pub y_pos: Property<Size>,
    pub signal: Property<bool>,
    pub on_message_changed: Property<bool>,
}

impl Toast {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.height.set(Size::Pixels(80.into()));
        self.y_pos.set(Size::default() + self.height.get());
        let message = self.message.clone();
        let deps = [message.untyped()];
        let y_pos = self.y_pos.clone();
        let height = self.height.clone();
        self.shown.set(true);
        self.on_message_changed.replace_with(Property::computed(
            move || {
                if message.get() != "" {
                    // show for the animation period
                    // in
                    set_px_offset(&y_pos, Size::ZERO(), IN_OUT_TIME);
                    // stay
                    set_px_offset_later(&y_pos, Size::ZERO(), 300);
                    // out
                    set_px_offset_later(&y_pos, height.get(), IN_OUT_TIME);
                }
                false
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirtification
        self.on_message_changed.get();
    }

    pub fn handle_trigger(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        set_px_offset(&self.y_pos, self.height.get(), IN_OUT_TIME);
        self.signal.set(true);
    }
}

fn set_px_offset(y_pos: &Property<Size>, offset: Size, time: u64) {
    y_pos.ease_to(Size::default() + offset, time, EasingCurve::InQuad);
}

fn set_px_offset_later(y_pos: &Property<Size>, offset: Size, time: u64) {
    y_pos.ease_to_later(Size::default() + offset, time, EasingCurve::OutQuad);
}
