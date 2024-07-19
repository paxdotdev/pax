#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Calculator {
    pub display: Property<String>,
    pub first_number: Property<f64>,
    pub operation: Property<String>,
    pub second_number: Property<f64>,
}

impl Calculator {
    pub fn handle_number(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>, number: u8) {
        let mut current_display = self.display.get();
        if current_display == "0" {
            current_display = number.to_string();
        } else {
            current_display.push_str(&number.to_string());
        }
        self.display.set(current_display);
    }

    pub fn handle_operation(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>, op: &str) {
        self.first_number.set(self.display.get().parse().unwrap_or(0.0));
        self.operation.set(op.to_string());
        self.display.set("0".to_string());
    }

    pub fn handle_equals(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>) {
        self.second_number.set(self.display.get().parse().unwrap_or(0.0));
        let result = match self.operation.get().as_str() {
            "+" => self.first_number.get() + self.second_number.get(),
            "-" => self.first_number.get() - self.second_number.get(),
            "*" => self.first_number.get() * self.second_number.get(),
            "/" => self.first_number.get() / self.second_number.get(),
            _ => self.second_number.get(),
        };
        self.display.set(result.to_string());
    }

    pub fn handle_clear(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>) {
        self.display.set("0".to_string());
        self.first_number.set(0.0);
        self.operation.set("".to_string());
        self.second_number.set(0.0);
    }

    pub fn handle0(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 0); }
    pub fn handle1(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 1); }
    pub fn handle2(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 2); }
    pub fn handle3(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 3); }
    pub fn handle4(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 4); }
    pub fn handle5(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 5); }
    pub fn handle6(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 6); }
    pub fn handle7(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 7); }
    pub fn handle8(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 8); }
    pub fn handle9(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_number(ctx, args, 9); }

    pub fn handleAdd(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "+"); }
    pub fn handleSubtract(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "-"); }
    pub fn handleMultiply(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "*"); }
    pub fn handleDivide(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "/"); }
}