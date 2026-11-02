#![allow(unused_imports)]

use pax_kit::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::HashSet;

pub mod animation;
use animation::Animation;

const SCALE: f64 = 2.0;
const TOUCH_SPEED: f64 = 1.5;
const BULLET_COOLDOWN: u64 = 15;

pax_kit::runnable! { root: crate::SpaceGame }

#[pax]
#[file("lib.pax")]
pub struct SpaceGame {
    pub ship_x: Property<f64>,
    pub ship_y: Property<f64>,
    pub asteroids: Property<Vec<Asteroid>>,
    pub bullets: Property<Vec<Bullet>>,

    pub last_asteroid: Property<u64>,
    pub last_bullet: Property<u64>,
    pub background_tiles: Property<Vec<BackgroundTile>>,

    pub keys_pressed: Property<Vec<u8>>,
    pub game_state: Property<String>,

    pub difficulty: Property<f64>,
    pub score: Property<u64>,
    pub game_over_scale: Property<f64>,
    pub auto_fire: Property<bool>,
    pub active_touch: Property<Option<i64>>,
    pub touch_x: Property<f64>,
    pub touch_y: Property<f64>,
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
pub struct BackgroundTile {
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
                ImageSource::Url(String::from("assets/asteroid0.png")),
                ImageSource::Url(String::from("assets/asteroid1.png")),
                ImageSource::Url(String::from("assets/asteroid2.png")),
            ]),
        }
    }
}

impl SpaceGame {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.auto_fire.set(ctx.os.is_mobile());
        let bounds = ctx.bounds_self.clone();
        let game_over_scale = self.game_over_scale.clone();
        ctx.subscribe(&[bounds.untyped()], move || {
            let (width, height) = bounds.get();
            game_over_scale.set((width / 500.0).min(height / 560.0).clamp(0.32, 1.0));
        });
        self.start_round(ctx.bounds_self.get().1, ctx.elapsed_frames.get());
    }

    pub fn play_again(&mut self, ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.start_round(ctx.bounds_self.get().1, ctx.elapsed_frames.get());
    }

    fn start_round(&mut self, viewport_height: f64, frame: u64) {
        self.ship_x.set(32.0);
        self.ship_y.set(viewport_height / SCALE / 2.0);
        self.asteroids.set(Vec::new());
        self.bullets.set(Vec::new());
        self.keys_pressed.set(Vec::new());
        self.active_touch.set(None);
        self.score.set(0);
        self.difficulty.set(0.0);
        // The runtime clock keeps advancing between rounds; restart cooldowns from now.
        self.last_asteroid.set(frame);
        self.last_bullet.set(frame);
        self.game_state.set(String::from("PLAYING"));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let mut rng = rand::thread_rng();

        // Read properties
        let (w_o, h_o) = ctx.bounds_self.get();
        let (w, h) = (w_o / SCALE, h_o / SCALE);
        let ticks = ctx.elapsed_frames.get();
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
                    self.active_touch.set(None);
                    a.animation.start();
                }
            }
        }

        if &self.game_state.get() == "PLAYING" {
            // Touch owns movement while held; keyboard remains unchanged otherwise.
            if self.active_touch.get().is_some() {
                (ship_x, ship_y) = move_toward(
                    (ship_x, ship_y),
                    (self.touch_x.get() * w, self.touch_y.get() * h),
                    TOUCH_SPEED,
                );
            } else {
                for key in self.keys_pressed.get() {
                    match key as char {
                        'w' => ship_y -= 1.5,
                        's' => ship_y += 1.5,
                        'a' => ship_x -= 1.0,
                        'd' => ship_x += 0.8,
                        _ => (),
                    }
                }
            }
            ship_x = ship_x.clamp(16.0, (w - 16.0).max(16.0));
            ship_y = ship_y.clamp(16.0, (h - 16.0).max(16.0));
            self.fire_if_ready(ticks, (ship_x, ship_y), &mut bullets);
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

        // Update tile background positions
        const BACKGROUND_SPEED: f64 = 5.0;
        const IMG_SIZE: f64 = 512.0;
        let w_n = (w_o / IMG_SIZE).ceil() as usize + 1;
        let h_n = (h_o / IMG_SIZE).ceil() as usize;
        let mut backgrounds = vec![];
        for j in 0..h_n {
            for i in 0..w_n {
                backgrounds.push(BackgroundTile {
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

    fn fire_if_ready(&mut self, ticks: u64, ship: (f64, f64), bullets: &mut Vec<Bullet>) {
        if self.game_state.get() == "PLAYING"
            && (self.auto_fire.get() || self.keys_pressed.get().contains(&b' '))
            && ticks.saturating_sub(self.last_bullet.get()) > BULLET_COOLDOWN
        {
            for offset in [-6.0, 6.0] {
                bullets.push(Bullet {
                    x: ship.0,
                    y: ship.1 + offset,
                });
            }
            self.last_bullet.set(ticks);
        }
    }

    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            if self.begin_touch(touch.identifier) {
                self.update_touch_target(ctx, touch);
            }
        }
    }

    fn begin_touch(&mut self, identifier: i64) -> bool {
        // Touch-capable desktops (and tablets reporting a desktop OS) opt in too.
        self.auto_fire.set(true);
        if self.game_state.get() != "PLAYING" || self.active_touch.get().is_some() {
            return false;
        }
        self.active_touch.set(Some(identifier));
        true
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(touch) = event
            .touches
            .iter()
            .find(|touch| self.active_touch.get() == Some(touch.identifier))
        {
            self.update_touch_target(ctx, touch);
            event.prevent_default();
        }
    }

    fn update_touch_target(&mut self, ctx: &NodeContext, touch: &Touch) {
        let local = ctx.local_point(pax_kit::api::math::Point2::new(touch.x, touch.y));
        // Keep the target normalized across resizing and the game's 2x sprite scale.
        self.touch_x.set(local.x.clamp(0.0, 1.0));
        self.touch_y.set(local.y.clamp(0.0, 1.0));
    }

    pub fn touch_end(&mut self, _ctx: &NodeContext, event: Event<TouchEnd>) {
        self.release_touch(&event.touches);
    }

    pub fn touch_cancel(&mut self, _ctx: &NodeContext, event: Event<TouchCancel>) {
        self.release_touch(&event.touches);
    }

    fn release_touch(&mut self, touches: &[Touch]) {
        if touches
            .iter()
            .any(|touch| self.active_touch.get() == Some(touch.identifier))
        {
            self.active_touch.set(None);
        }
    }

    pub fn key_down(&mut self, _ctx: &NodeContext, args: Event<KeyDown>) {
        if let Some(char) = args.keyboard.key.chars().next() {
            let mut keys_pressed = self.keys_pressed.get();
            if !keys_pressed.contains(&(char as u8)) {
                keys_pressed.push(char as u8);
            }
            self.keys_pressed.set(keys_pressed);
        }
    }
    pub fn key_up(&mut self, _ctx: &NodeContext, args: Event<KeyUp>) {
        if let Some(char) = args.keyboard.key.chars().next() {
            let mut keys_pressed = self.keys_pressed.get();
            keys_pressed.retain(|v| v != &(char as u8));
            self.keys_pressed.set(keys_pressed);
        }
    }
}

fn move_toward(position: (f64, f64), target: (f64, f64), speed: f64) -> (f64, f64) {
    let (dx, dy) = (target.0 - position.0, target.1 - position.1);
    let distance = dx.hypot(dy);
    if distance <= speed {
        target
    } else {
        (
            position.0 + dx / distance * speed,
            position.1 + dy / distance * speed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_a_round_clears_previous_gameplay_and_restarts_cooldowns() {
        let mut game = SpaceGame::default();

        // Exercise repeated restarts after the app has been running for a while.
        for frame in [10_000, 20_000] {
            game.game_state.set(String::from("GAME_OVER"));
            game.ship_x.set(200.0);
            game.ship_y.set(100.0);
            game.asteroids.set(vec![Asteroid::default()]);
            game.bullets.set(vec![Bullet { x: 100.0, y: 50.0 }]);
            game.keys_pressed.set(vec![b'w', b' ']);
            game.active_touch.set(Some(7));
            game.auto_fire.set(true);
            game.score.set(12);
            game.difficulty.set(1.0);
            game.last_asteroid.set(frame - 300);
            game.last_bullet.set(frame - 100);

            game.start_round(600.0, frame);

            assert_eq!(game.game_state.get(), "PLAYING");
            assert_eq!(game.ship_x.get(), 32.0);
            assert_eq!(game.ship_y.get(), 150.0);
            assert!(game.asteroids.get().is_empty());
            assert!(game.bullets.get().is_empty());
            assert!(game.keys_pressed.get().is_empty());
            assert_eq!(game.active_touch.get(), None);
            assert!(game.auto_fire.get());
            assert_eq!(game.score.get(), 0);
            assert_eq!(game.difficulty.get(), 0.0);
            assert_eq!(game.last_asteroid.get(), frame);
            assert_eq!(game.last_bullet.get(), frame);
        }
    }

    #[test]
    fn touch_movement_has_constant_speed_and_never_overshoots() {
        for target in [(30.0, 40.0), (-30.0, 40.0), (0.0, -50.0)] {
            let position = move_toward((0.0, 0.0), target, TOUCH_SPEED);
            assert!((position.0.hypot(position.1) - TOUCH_SPEED).abs() < 1e-10);
            assert!(position.0 * target.0 + position.1 * target.1 > 0.0);
        }
        assert_eq!(move_toward((0.0, 0.0), (0.3, 0.4), TOUCH_SPEED), (0.3, 0.4));
        assert_eq!(move_toward((5.0, 6.0), (5.0, 6.0), TOUCH_SPEED), (5.0, 6.0));
    }

    #[test]
    fn only_the_captured_finger_can_steer_or_release() {
        let mut game = SpaceGame::default();
        game.start_round(600.0, 0);
        assert!(game.begin_touch(7));
        assert!(game.auto_fire.get());
        assert!(!game.begin_touch(8));
        assert_eq!(game.active_touch.get(), Some(7));
        let touch = |identifier| Touch {
            identifier,
            x: 0.0,
            y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
        };
        game.release_touch(&[touch(8)]);
        assert_eq!(game.active_touch.get(), Some(7));
        // End and cancellation share this same release path.
        game.release_touch(&[touch(7)]);
        assert_eq!(game.active_touch.get(), None);
        assert!(game.auto_fire.get());
        game.game_state.set("GAME_OVER".into());
        assert!(!game.begin_touch(9));
        assert_eq!(game.active_touch.get(), None);
    }

    #[test]
    fn auto_fire_continues_without_a_touch_and_respects_cooldown_and_game_over() {
        let mut game = SpaceGame::default();
        game.start_round(600.0, 100);
        game.auto_fire.set(true);
        let mut bullets = vec![];
        game.fire_if_ready(115, (32.0, 150.0), &mut bullets);
        assert!(bullets.is_empty());
        game.fire_if_ready(116, (32.0, 150.0), &mut bullets);
        assert_eq!(bullets.len(), 2);
        assert_eq!((bullets[0].x, bullets[0].y), (32.0, 144.0));
        game.fire_if_ready(117, (32.0, 150.0), &mut bullets);
        assert_eq!(bullets.len(), 2);
        game.fire_if_ready(132, (32.0, 150.0), &mut bullets);
        assert_eq!(bullets.len(), 4);
        game.game_state.set("GAME_OVER".into());
        game.fire_if_ready(148, (32.0, 150.0), &mut bullets);
        assert_eq!(bullets.len(), 4);
    }

    #[test]
    fn keyboard_firing_still_requires_space_until_touch_mode_is_enabled() {
        let mut game = SpaceGame::default();
        game.start_round(600.0, 0);
        let mut bullets = vec![];
        game.fire_if_ready(16, (32.0, 150.0), &mut bullets);
        assert!(bullets.is_empty());
        game.keys_pressed.set(vec![b' ']);
        game.fire_if_ready(16, (32.0, 150.0), &mut bullets);
        assert_eq!(bullets.len(), 2);
    }
}
