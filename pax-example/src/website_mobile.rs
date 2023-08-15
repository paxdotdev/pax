use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker, Sidebar};

#[derive(Pax)]
#[file("website_mobile.pax")]
pub struct WebsiteMobile {
    pub scroll_position: Property<f64>,
    pub scroll_range: Property<f64>,
    pub touch_down: Property<bool>,
    pub current_touch_point: Property<f64>,
    pub first_touch_point: Property<f64>,
    pub old_touch_point: Property<f64>,
    pub frame_start: Property<usize>,
    pub velocity: Property<f64>,
}


impl WebsiteMobile {

    pub fn handle_container_scroll(&mut self, ctx: RuntimeContext, args: ArgsScroll) {
        let mut scroll_position = *self.scroll_position.get();
        scroll_position = scroll_position - args.delta_y;
        scroll_position = scroll_position.min(0.0);
        scroll_position = scroll_position.max(-3400.0);
        self.scroll_position.set(scroll_position);
    }

    pub fn handle_touch_start(&mut self, ctx: RuntimeContext, args: ArgsTouchStart){
        // let touch = args.touches.first().unwrap();
        // self.touch_down.set(true);
        // self.current_touch_point.set(touch.y);
        // self.first_touch_point.set(touch.y);
        // self.old_touch_point.set(touch.y);
        // self.frame_start.set(ctx.frames_elapsed);
    }
    pub fn handle_touch_move(&mut self, ctx: RuntimeContext, args: ArgsTouchMove){
        // let touch = args.touches.first().unwrap();
        // self.current_touch_point.set(touch.y);
        // let diff = (touch.y - self.scroll_position.get());
        // let move_direction = diff/(diff.abs());
        // let velocity_direction = self.velocity.get()/(self.velocity.get().abs());
        // if move_direction != velocity_direction {
        //     self.velocity.set(0.0);
        // }
    }
    pub fn handle_touch_end(&mut self, ctx: RuntimeContext, args: ArgsTouchEnd){
        // let touch = args.touches.first().unwrap();
        // self.touch_down.set(false);
        // let velocity = (self.current_touch_point.get() - self.first_touch_point.get())/((ctx.frames_elapsed- self.frame_start.get()) as f64);
        // let last_move = touch.y - self.old_touch_point.get();
        // if last_move.abs() > 5.0 {
        //     self.velocity.set(velocity);
        // }

    }
    pub fn handle_will_render(&mut self, ctx: RuntimeContext){
        // let mut new_position = *self.scroll_position.get();
        // if *self.touch_down.get() {
        //     new_position += self.current_touch_point.get() - self.old_touch_point.get();
        //     self.old_touch_point.set(*self.current_touch_point.get());
        //     new_position = new_position.min(0.0).max(-3400.0);
        //     self.scroll_position.set(new_position);
        // } else {
        //     let mut velocity = *self.velocity.get();
        //     if velocity.abs() > 1.0 {
        //         new_position += velocity;
        //         self.velocity.set(velocity*0.95);
        //         new_position = new_position.min(0.0).max(-3400.0);
        //         self.scroll_position.set(new_position);
        //     }
        // }
    }
}