#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::*;
use pax_std::types::text::*;
use pax_std::types::*;

const PADDLE_SPEED: f64 = 5.0;
const BALL_SPEED: f64 = 5.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Pong {
    pub player_y: Property<f64>,
    pub computer_y: Property<f64>,
    pub ball_x: Property<f64>,
    pub ball_y: Property<f64>,
    pub ball_dx: Property<f64>,
    pub ball_dy: Property<f64>,
    pub player_score: Property<u32>,
    pub computer_score: Property<u32>,
}

impl Pong {
    pub fn update_game(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();

        // Update ball position
        let ball_x = self.ball_x.get() + self.ball_dx.get();
        let ball_y = self.ball_y.get() + self.ball_dy.get();

        // Ball collision with top and bottom walls
        if ball_y <= 0.0 || ball_y >= height - 10.0 {
            self.ball_dy.set(-self.ball_dy.get());
        }

        // Ball collision with paddles
        if (ball_x <= 30.0 && ball_y >= self.player_y.get() && ball_y <= self.player_y.get() + 80.0) ||
           (ball_x >= width - 40.0 && ball_y >= self.computer_y.get() && ball_y <= self.computer_y.get() + 80.0) {
            self.ball_dx.set(-self.ball_dx.get());
        }

        // Score points
        if ball_x <= 0.0 {
            self.computer_score.set(self.computer_score.get() + 1);
            self.reset_ball(ctx);
        } else if ball_x >= width - 10.0 {
            self.player_score.set(self.player_score.get() + 1);
            self.reset_ball(ctx);
        } else {
            self.ball_x.set(ball_x);
            self.ball_y.set(ball_y);
        }

        // Simple AI for computer paddle
        let computer_y = self.computer_y.get();
        if computer_y + 40.0 < ball_y {
            self.computer_y.set(computer_y + PADDLE_SPEED);
        } else if computer_y + 40.0 > ball_y {
            self.computer_y.set(computer_y - PADDLE_SPEED);
        }
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        let (_, height) = ctx.bounds_self.get();
        self.player_y.set((args.mouse.y - 40.0).clamp(0.0, height - 80.0));
    }

    pub fn reset_ball(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        self.ball_x.set(width / 2.0 - 5.0);
        self.ball_y.set(height / 2.0 - 5.0);
        self.ball_dx.set(if rand::random() { BALL_SPEED } else { -BALL_SPEED });
        self.ball_dy.set(if rand::random() { BALL_SPEED } else { -BALL_SPEED });
    }
}