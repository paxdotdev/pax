#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use rand::Rng;

const GAME_DURATION: u64 = 60; // 60 seconds game duration

#[pax]
#[main]
#[file("lib.pax")]
pub struct Katamari {
    pub ball_x: Property<f64>,
    pub ball_y: Property<f64>,
    pub ball_size: Property<f64>,
    pub objects: Property<Vec<GameObject>>,
    pub score: Property<u64>,
    pub time_left: Property<u64>,
    pub game_over: Property<bool>,
    pub keys_pressed: Property<Vec<String>>,
}

#[pax]
pub struct GameObject {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub color: Color,
}

impl Katamari {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        self.ball_x.set(width / 2.0);
        self.ball_y.set(height / 2.0);
        self.ball_size.set(20.0);
        self.score.set(0);
        self.time_left.set(GAME_DURATION);
        self.game_over.set(false);

        self.spawn_objects(width, height);
    }

    fn spawn_objects(&mut self, width: f64, height: f64) {
        let mut rng = rand::thread_rng();
        let mut objects = self.objects.get();

        for _ in 0..50 {
            objects.push(GameObject {
                x: rng.gen_range(0.0..width),
                y: rng.gen_range(0.0..height),
                size: rng.gen_range(5.0..15.0),
                color: Color::rgb(
                    rng.gen_range(0..255).into(),
                    rng.gen_range(0..255).into(),
                    rng.gen_range(0..255).into(),
                ),
            });
        }

        self.objects.set(objects);
    }

    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        if self.game_over.get() {
            return;
        }

        let elapsed = ctx.frames_elapsed.get();
        self.time_left.set(GAME_DURATION.saturating_sub(elapsed / 60));

        if self.time_left.get() == 0 {
            self.game_over.set(true);
            return;
        }

        let (width, height) = ctx.bounds_self.get();
        let mut ball_x = self.ball_x.get();
        let mut ball_y = self.ball_y.get();
        let ball_size = self.ball_size.get();

        // Move ball based on pressed keys
        for key in self.keys_pressed.get().iter() {
            match key.as_str() {
                "ArrowUp" => ball_y -= 5.0,
                "ArrowDown" => ball_y += 5.0,
                "ArrowLeft" => ball_x -= 5.0,
                "ArrowRight" => ball_x += 5.0,
                _ => {}
            }
        }

        // Keep ball within bounds
        ball_x = ball_x.clamp(0.0, width - ball_size);
        ball_y = ball_y.clamp(0.0, height - ball_size);

        self.ball_x.set(ball_x);
        self.ball_y.set(ball_y);

        // Check collisions and update objects
        let mut score = self.score.get();
        let mut new_ball_size = ball_size;

        self.objects.update(|objects| {
            objects.retain(|obj| {
                let dx = ball_x + ball_size / 2.0 - (obj.x + obj.size / 2.0);
                let dy = ball_y + ball_size / 2.0 - (obj.y + obj.size / 2.0);
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < (ball_size + obj.size) / 2.0 && ball_size > obj.size {
                    score += 1;
                    new_ball_size += obj.size / 10.0;
                    false
                } else {
                    true
                }
            });

            // Spawn new objects if needed
            if objects.len() < 20 {
                let mut rng = rand::thread_rng();
                for _ in 0..(50 - objects.len()) {
                    objects.push(GameObject {
                        x: rng.gen_range(0.0..width),
                        y: rng.gen_range(0.0..height),
                        size: rng.gen_range(5.0..15.0),
                        color: Color::rgb(
                            rng.gen_range(0..255).into(),
                            rng.gen_range(0..255).into(),
                            rng.gen_range(0..255).into(),
                        ),
                    });
                }
            }
        });

        self.score.set(score);
        self.ball_size.set(new_ball_size);
    }

    pub fn handle_key_down(&mut self, _ctx: &NodeContext, args: Event<KeyDown>) {
        let mut keys = self.keys_pressed.get();
        if !keys.contains(&args.keyboard.key) {
            keys.push(args.keyboard.key.clone());
            self.keys_pressed.set(keys);
        }
    }

    pub fn handle_key_up(&mut self, _ctx: &NodeContext, args: Event<KeyUp>) {
        let mut keys = self.keys_pressed.get();
        keys.retain(|key| key != &args.keyboard.key);
        self.keys_pressed.set(keys);
    }
}