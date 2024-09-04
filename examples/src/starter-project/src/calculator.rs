#![allow(unused_imports)]
use pax_kit::*;

#[pax]
#[file("calculator.pax")]
pub struct Calculator {
    pub display: Property<String>,
    pub current_operation: Property<String>,
    pub previous_value: Property<f64>,
    pub is_new_input: Property<bool>,
}

#[pax]
pub struct PathConfig {
    pub amplitude: Property<Numeric>,
    pub amplitude_ramp: Property<Numeric>,
    pub frequency: Property<Numeric>,
    pub frequency_ramp: Property<Numeric>,
    pub thickness: Property<Numeric>,
    pub thickness_ramp: Property<Numeric>,
    pub span: Property<Numeric>,
}

impl Calculator {
    pub fn handle_number(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>, number: char) {
        if self.is_new_input.get() {
            self.display.set(number.to_string());
            self.is_new_input.set(false);
        } else {
            let mut current = self.display.get();
            current.push(number);
            self.display.set(current);
        }
    }

    pub fn handle_operation(
        &mut self,
        _ctx: &NodeContext,
        _args: Event<ButtonClick>,
        operation: &str,
    ) {
        self.previous_value
            .set(self.display.get().parse().unwrap_or(0.0));
        self.current_operation.set(operation.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_equals(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let current_value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = match self.current_operation.get().as_str() {
            "+" => self.previous_value.get() + current_value,
            "-" => self.previous_value.get() - current_value,
            "×" => self.previous_value.get() * current_value,
            "÷" => self.previous_value.get() / current_value,
            "^" => self.previous_value.get().powf(current_value),
            _ => current_value,
        };
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_clear(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.display.set("0".to_string());
        self.current_operation.set("".to_string());
        self.previous_value.set(0.0);
        self.is_new_input.set(true);
    }

    pub fn handle0(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '0');
    }
    pub fn handle1(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '1');
    }
    pub fn handle2(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '2');
    }
    pub fn handle3(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '3');
    }
    pub fn handle4(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '4');
    }
    pub fn handle5(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '5');
    }
    pub fn handle6(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '6');
    }
    pub fn handle7(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '7');
    }
    pub fn handle8(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '8');
    }
    pub fn handle9(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_number(ctx, args, '9');
    }

    pub fn handle_add(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_operation(ctx, args, "+");
    }
    pub fn handle_subtract(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_operation(ctx, args, "-");
    }
    pub fn handle_multiply(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_operation(ctx, args, "×");
    }
    pub fn handle_divide(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_operation(ctx, args, "÷");
    }
    pub fn handle_power(&mut self, ctx: &NodeContext, args: Event<ButtonClick>) {
        self.handle_operation(ctx, args, "^");
    }

    pub fn handle_sin(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.to_radians().sin();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_cos(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.to_radians().cos();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_tan(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.to_radians().tan();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_log(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.log10();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_ln(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.ln();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_sqrt(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        let value: f64 = self.display.get().parse().unwrap_or(0.0);
        let result = value.sqrt();
        self.display.set(result.to_string());
        self.is_new_input.set(true);
    }

    pub fn handle_pi(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.display.set(std::f64::consts::PI.to_string());
        self.is_new_input.set(true);
    }
}
