#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct ScientificCalculator {
    pub display: Property<String>,
    pub current_input: Property<String>,
    pub result: Property<f64>,
    pub operation: Property<String>,
}

impl ScientificCalculator {
    pub fn handle_number(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>, number: u8) {
        let mut current_input = self.current_input.get();
        current_input.push_str(&number.to_string());
        self.current_input.set(current_input.clone());
        self.display.set(current_input);
    }

    pub fn handle_operation(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>, op: &str) {
        let current_input = self.current_input.get();
        if !current_input.is_empty() {
            self.result.set(current_input.parse().unwrap_or(0.0));
        }
        self.operation.set(op.to_string());
        self.current_input.set(String::new());
    }

    pub fn handle_equals(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>) {
        let current_input = self.current_input.get();
        let second_number: f64 = current_input.parse().unwrap_or(0.0);
        let result = match self.operation.get().as_str() {
            "+" => self.result.get() + second_number,
            "-" => self.result.get() - second_number,
            "*" => self.result.get() * second_number,
            "/" => self.result.get() / second_number,
            "^" => self.result.get().powf(second_number),
            _ => second_number,
        };
        self.result.set(result);
        self.display.set(result.to_string());
        self.current_input.set(String::new());
    }

    pub fn handle_clear(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>) {
        self.display.set("0".to_string());
        self.current_input.set(String::new());
        self.result.set(0.0);
        self.operation.set(String::new());
    }

    pub fn handle_function(&mut self, _ctx: &NodeContext, args: Event<ButtonClick>, func: &str) {
        let current_input = self.current_input.get();
        let input: f64 = current_input.parse().unwrap_or(0.0);
        let result = match func {
            "sqrt" => input.sqrt(),
            "sin" => input.to_radians().sin(),
            "cos" => input.to_radians().cos(),
            "tan" => input.to_radians().tan(),
            "log" => input.log10(),
            _ => input,
        };
        self.result.set(result);
        self.display.set(result.to_string());
        self.current_input.set(String::new());
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

    pub fn handle_add(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "+"); }
    pub fn handle_subtract(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "-"); }
    pub fn handle_multiply(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "*"); }
    pub fn handle_divide(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "/"); }
    pub fn handle_power(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_operation(ctx, args, "^"); }

    pub fn handle_sqrt(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_function(ctx, args, "sqrt"); }
    pub fn handle_sin(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_function(ctx, args, "sin"); }
    pub fn handle_cos(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_function(ctx, args, "cos"); }
    pub fn handle_tan(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_function(ctx, args, "tan"); }
    pub fn handle_log(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) { self.handle_function(ctx, args, "log"); }
}