#![allow(unused_imports)]
use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::f64::consts::PI;

const SQUARE_SIZE: f64 = 100.0;
const BALL_SIZE: f64 = SQUARE_SIZE / 4.0;
const BALL_SPEED: f64 = 6.0;
const NUM_BALLS: usize = 10;
const COLLISION_ANIMATION_DURATION: u64 = 20; // Duration of collision animation in frames

#[pax]
#[main]
#[file("lib.pax")]
pub struct RotatingSquareWithBalls {
    pub rotation: Property<f64>,
    pub balls: Property<Vec<Ball>>,
    pub square_size: Property<f64>,
    pub ball_size: Property<f64>,
}

#[pax]
pub struct Ball {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub hue: f64,
    pub size: f64,
    pub collision_animation: u64, // Countdown for collision animation
}

impl Ball {
    fn new(x: f64, y: f64, vx: f64, vy: f64, hue: f64) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            hue,
            size: BALL_SIZE,
            collision_animation: 0,
        }
    }
}

impl RotatingSquareWithBalls {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.square_size.set(SQUARE_SIZE);
        self.ball_size.set(BALL_SIZE);
        self.randomize_balls(ctx);
    }

    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_parent.get();

        // Update rotation
        let current_rotation = self.rotation.get();
        self.rotation.set((current_rotation + 0.5) % 360.0);

        // Update ball positions and handle collisions
        let mut balls = self.balls.get();

        // First, update positions and handle wall/square collisions
        for ball in balls.iter_mut() {
            // Update position
            ball.x += ball.vx;
            ball.y += ball.vy;

            // Bounce off viewport boundaries
            if ball.x - BALL_SIZE / 2.0 < 0.0 || ball.x + BALL_SIZE / 2.0 > width {
                ball.vx = -ball.vx;
                ball.collision_animation = COLLISION_ANIMATION_DURATION;
            }
            if ball.y - BALL_SIZE / 2.0 < 0.0 || ball.y + BALL_SIZE / 2.0 > height {
                ball.vy = -ball.vy;
                ball.collision_animation = COLLISION_ANIMATION_DURATION;
            }

            // Check collision with rotating square
            let square_x = width / 2.0;
            let square_y = height / 2.0;
            let dx = ball.x - square_x;
            let dy = ball.y - square_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < SQUARE_SIZE / 2.0 + BALL_SIZE / 2.0 {
                // Collision detected, calculate bounce
                let angle = dy.atan2(dx);
                let normal_angle = angle - current_rotation.to_radians();

                let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
                ball.vx = speed * normal_angle.cos();
                ball.vy = speed * normal_angle.sin();

                // Move ball outside the square
                ball.x = square_x + (SQUARE_SIZE / 2.0 + BALL_SIZE / 2.0) * angle.cos();
                ball.y = square_y + (SQUARE_SIZE / 2.0 + BALL_SIZE / 2.0) * angle.sin();

                ball.collision_animation = COLLISION_ANIMATION_DURATION;
            }

            // Update ball color and size
            ball.hue = (ball.hue + 0.5) % 360.0;
            if ball.collision_animation > 0 {
                ball.collision_animation -= 1;
                let progress =
                    ball.collision_animation as f64 / COLLISION_ANIMATION_DURATION as f64;
                ball.size = BALL_SIZE * (1.0 + 0.2 * (1.0 - progress).powi(2));
            } else {
                ball.size = BALL_SIZE;
            }
        }

        // Then, handle ball-to-ball collisions
        for i in 0..balls.len() {
            for j in (i + 1)..balls.len() {
                let dx = balls[j].x - balls[i].x;
                let dy = balls[j].y - balls[i].y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < BALL_SIZE {
                    // Collision detected, calculate new velocities
                    let angle = dy.atan2(dx);
                    let sin = angle.sin();
                    let cos = angle.cos();

                    // Rotate velocities
                    let vx1 = balls[i].vx * cos + balls[i].vy * sin;
                    let vy1 = balls[i].vy * cos - balls[i].vx * sin;
                    let vx2 = balls[j].vx * cos + balls[j].vy * sin;
                    let vy2 = balls[j].vy * cos - balls[j].vx * sin;

                    // Swap velocities
                    let (new_vx1, new_vx2) = (vx2, vx1);

                    // Rotate velocities back
                    balls[i].vx = new_vx1 * cos - vy1 * sin;
                    balls[i].vy = vy1 * cos + new_vx1 * sin;
                    balls[j].vx = new_vx2 * cos - vy2 * sin;
                    balls[j].vy = vy2 * cos + new_vx2 * sin;

                    // Move balls apart to prevent sticking
                    let overlap = BALL_SIZE - distance;
                    let move_x = overlap * cos / 2.0;
                    let move_y = overlap * sin / 2.0;
                    balls[i].x -= move_x;
                    balls[i].y -= move_y;
                    balls[j].x += move_x;
                    balls[j].y += move_y;

                    // Start collision animation
                    balls[i].collision_animation = COLLISION_ANIMATION_DURATION;
                    balls[j].collision_animation = COLLISION_ANIMATION_DURATION;

                    // Change hue on collision
                    balls[i].hue = (balls[i].hue + 60.0) % 360.0;
                    balls[j].hue = (balls[j].hue + 60.0) % 360.0;
                }
            }
        }

        self.balls.set(balls);
    }

    pub fn handle_click(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.randomize_balls(ctx);
    }

    fn randomize_balls(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_parent.get();
        let seed = ctx.frames_elapsed.get() as u64;
        let mut rng = StdRng::seed_from_u64(seed);

        let mut new_balls = Vec::new();

        for _ in 0..NUM_BALLS {
            loop {
                let x = rng.gen_range(BALL_SIZE / 2.0..width - BALL_SIZE / 2.0);
                let y = rng.gen_range(BALL_SIZE / 2.0..height - BALL_SIZE / 2.0);

                // Ensure the ball doesn't overlap with the square
                if (x - width / 2.0).abs() > SQUARE_SIZE / 2.0 + BALL_SIZE / 2.0
                    || (y - height / 2.0).abs() > SQUARE_SIZE / 2.0 + BALL_SIZE / 2.0
                {
                    let angle = rng.gen_range(0.0..2.0 * PI);
                    new_balls.push(Ball::new(
                        x,
                        y,
                        BALL_SPEED * angle.cos(),
                        BALL_SPEED * angle.sin(),
                        rng.gen_range(0.0..360.0),
                    ));
                    break;
                }
            }
        }

        self.balls.set(new_balls);
    }
}
