#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashSet;

pub mod animation;
use animation::Animation;

const SCALE: f64 = 2.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct SpaceGame {
    pub ship_x: Property<f64>,
    pub ship_y: Property<f64>,
    pub asteroids: Property<Vec<Asteroid>>,
    pub bullets: Property<Vec<Bullet>>,

    pub last_asteroid: Property<u64>,
    pub last_bullet: Property<u64>,
    pub background_tiles: Property<Vec<Point>>,

    pub keys_pressed: Property<Vec<u8>>,
    pub game_state: Property<String>,

    pub difficulty: Property<f64>,
    pub score: Property<u64>,
    pub score_text: Property<String>,
}

#[pax]
pub struct Asteroid {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub dx: f64,
    pub dy: f64,
    pub dr: f64,
    pub w: f64,
    pub health: u64,
    pub animation: Animation,
}

#[pax]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[pax]
pub struct Bullet {
    pub x: f64,
    pub y: f64,
}

impl Asteroid {
    fn spawn(rng: &mut ThreadRng, w: f64, h: f64, difficulty: f64) -> Self {
        let radius = rng.gen_range(32.0..(48.0 + 32.0 * difficulty));
        let yspread = 0.2 + difficulty * 0.3;
        Self {
            x: w + 32.0,
            y: rng.gen_range(-32.0..(h + 32.0)),
            r: rng.gen_range(0.0..360.0),
            dx: 1.0 + rng.gen_range((1.5 * difficulty)..(1.0 + 3.2 * difficulty)),
            dy: rng.gen_range(-yspread..yspread),
            dr: rng.gen_range(0.0..1.0),
            w: radius,
            health: (radius as u64) / 3,
            animation: Animation::new(vec![
                String::from("assets/asteroid0.png"),
                String::from("assets/asteroid1.png"),
                String::from("assets/asteroid2.png"),
            ]),
        }
    }
}

impl SpaceGame {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let mut rng = rand::thread_rng();
        let (w_o, h_o) = ctx.bounds_parent.get();
        let (w, h) = (w_o / SCALE, h_o / SCALE);
        self.ship_x.set(32.0);
        self.ship_y.set(h / 2.0);
        self.game_state.set(String::from("PLAYING"));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let mut rng = rand::thread_rng();

        // Read properties
        let (w_o, h_o) = ctx.bounds_parent.get();
        let (w, h) = (w_o / SCALE, h_o / SCALE);
        let ticks = ctx.frames_elapsed.get();
        let mut bullets = self.bullets.get();
        let mut asteroids = self.asteroids.get();
        let mut ship_x = self.ship_x.get();
        let mut ship_y = self.ship_y.get();
        let mut score = self.score.get();
        let difficulty = self.difficulty.get();

        if &self.game_state.get() == "PLAYING" {
            // Check collisions between player and asteroids, and end game if hit
            for a in &mut asteroids {
                if (a.x - ship_x).powi(2) + (a.y - ship_y).powi(2) < (10.0f64 + a.w / 2.0).powi(2) {
                    self.game_state.set(String::from("GAME_OVER"));
                    a.animation.start();
                }
            }

            // Player actions (movement, bullets, etc.)
            for key in self.keys_pressed.get() {
                match key as char {
                    'w' => ship_y -= 1.5,
                    's' => ship_y += 1.5,
                    'a' => ship_x -= 1.0,
                    'd' => ship_x += 0.8,
                    ' ' => {
                        if (ticks - self.last_bullet.get()) > 15 {
                            bullets.push(Bullet {
                                x: ship_x,
                                y: ship_y + 6.0,
                            });
                            bullets.push(Bullet {
                                x: ship_x,
                                y: ship_y - 6.0,
                            });
                            self.last_bullet.set(ticks);
                        }
                    }
                    _ => (),
                }
            }
            ship_x = ship_x.clamp(16.0, w - 16.0);
            ship_y = ship_y.clamp(16.0, h - 16.0);
        }

        // Update bullets (movement, destroy, create, etc)
        bullets.retain_mut(|b| {
            b.x += 7.0;
            // Collide with asteroids
            for a in &mut asteroids {
                if (a.x - b.x).powi(2) + (a.y - b.y).powi(2) < (a.w / 2.0).powi(2) {
                    a.health = a.health.saturating_sub(1);
                    if a.health == 0 && !a.animation.running {
                        a.animation.start();
                        score += 1;
                    }
                    return false;
                }
            }
            b.x <= w + 16.0
        });

        // Update asteroids (movement, destroy, create, etc)
        asteroids.retain_mut(|a| {
            a.x -= a.dx;
            a.y -= a.dy;
            a.r += a.dr;
            a.animation.tick();
            a.x > -32.0 && a.animation.finished == false
        });
        let l_a = self.last_asteroid.get();
        if rng.gen_range(0..(ticks - l_a + 1)) > ((1.2 - difficulty) * 100.0) as u64 {
            self.last_asteroid.set(ticks);
            asteroids.push(Asteroid::spawn(&mut rng, w, h, difficulty));
        }

        // Update all properties to new values
        self.asteroids.set(asteroids);
        self.ship_x.set(ship_x);
        self.ship_y.set(ship_y);
        self.bullets.set(bullets);
        self.score.set(score);
        self.score_text.set(format!("Score: {}", score));

        // Update tile background positions
        const BACKGROUND_SPEED: f64 = 5.0;
        const IMG_SIZE: f64 = 512.0;
        let w_n = (w_o / IMG_SIZE).ceil() as usize + 1;
        let h_n = (h_o / IMG_SIZE).ceil() as usize;
        let mut backgrounds = vec![];
        for j in 0..h_n {
            for i in 0..w_n {
                backgrounds.push(Point {
                    x: (w_n as f64 - 1.0) * IMG_SIZE
                        - (ticks as f64 * BACKGROUND_SPEED + i as f64 * IMG_SIZE)
                            .rem_euclid(w_n as f64 * IMG_SIZE),
                    y: IMG_SIZE * j as f64,
                });
            }
        }
        self.background_tiles.set(backgrounds);
        self.difficulty.set((difficulty + 0.0001).min(1.0));
    }

    pub fn key_down(&mut self, ctx: &NodeContext, args: Event<KeyDown>) {
        if let Some(char) = args.keyboard.key.chars().next() {
            let mut keys_pressed = self.keys_pressed.get();
            if !keys_pressed.contains(&(char as u8)) {
                keys_pressed.push(char as u8);
            }
            self.keys_pressed.set(keys_pressed);
        }
    }
    pub fn key_up(&mut self, ctx: &NodeContext, args: Event<KeyUp>) {
        if let Some(char) = args.keyboard.key.chars().next() {
            let mut keys_pressed = self.keys_pressed.get();
            keys_pressed.retain(|v| v != &(char as u8));
            self.keys_pressed.set(keys_pressed);
        }
    }
}
