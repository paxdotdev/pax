use pax_engine::api::*;
use pax_engine::*;
use pax_std::primitives::*;
use std::f64::consts::PI;
use rand::Rng;
use rand_distr::{Distribution, Exp};

const INITIAL_PADDLE_WIDTH: f64 = 100.0;
const PADDLE_HEIGHT: f64 = 20.0;
const PADDLE_ELEVATION: f64 = 10.0;
const BALL_SIZE: f64 = 15.0;
const BRICK_WIDTH: f64 = 60.0;
const BRICK_HEIGHT: f64 = 20.0;
const BRICK_ROWS: usize = 5;
const BRICK_COLS: usize = 10;
const INITIAL_BALL_SPEED: f64 = 9.0;
const BALL_SPEED_MULTIPLIER: f64 = 1.50;
const MIN_BALL_DY: f64 = 2.0;
const MIN_PADDLE_WIDTH: f64 = 25.0;
const PADDLE_WIDTH_INCREMENT: f64 = 25.0;
const PADDLE_REVERSE_BOUNCE_PORTION: f64 = 0.05;
const APPROXIMATE_FPS: u64 = 60;
const RANDOMIZE_VELOCITY_MIN: f64 = PI/8.0; // in absolute value
const RANDOMIZE_VELOCITY_MAX: f64 = PI/2.0-PI/8.0; // in absolute value
const INTEREVENT_INF: u64 = 1; // measured in number of frames
const INTEREVENT_SUP: u64 = 4; // measured in number of frames
const SHOW_DEBUG_MARKERS: bool = false;

#[pax]
#[main]
#[file("lib.pax")]
pub struct BreakoutGame {
    pub balls: Property<Vec<Ball>>,
    pub bricks: Property<Vec<Brick>>,
    pub paddle: Property<Paddle>,
    pub score: Property<u32>,
    pub game_state: Property<u32>,
    pub brick_width: Property<f64>,
    pub brick_height: Property<f64>,
    pub background_fill: Property<Color>,
    pub debug_markers: Property<bool>,
}

#[pax]
pub struct Paddle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: Color,
}

#[pax]
pub struct Ball {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub radius: f64,
    pub fill: Color,
    pub visible: bool,  // used by event loop as despawn signal
    pub trigger: Option<ExpTrigger>,  // balls may randomly change direction
}

#[pax]
pub struct ExpTrigger {
    pub rate: f64,
    pub next_trigger: u64,
}

impl ExpTrigger {
    pub fn new(rate: f64, current_frame: u64) -> Self {
        let mut trigger = Self { rate: rate, next_trigger: 0 };
        trigger.schedule(current_frame);
        trigger
    }
    
    pub fn check(&mut self, current_frame: u64) -> bool {
        if current_frame >= self.next_trigger {
            self.schedule(current_frame);
            true
        } else {
            false
        }
    }

    fn schedule(&mut self, current_frame: u64) {
        let mut rng = rand::thread_rng();
        let exp = Exp::new(self.rate).unwrap();
        let interval = exp.sample(&mut rng) as u64;
        self.next_trigger = current_frame + interval;
    }
}

#[pax]
pub enum BrickType {
    #[default]
    NEUTRAL,
    SPAWN,
    PERTURB,
    ACCELERATE,
    LONGER,
    SHORTER,
}

#[pax]
pub struct Brick {
    pub x: f64,
    pub y: f64,
    pub visible: bool,  // used by event loop as despawn signal
    pub kind: BrickType,
    pub fill: Color,
}

#[pax]
pub enum Theme {
    #[default]
    CERULEAN,
    CURRANT,
    MARINE,
    LILAC,
    TEAL,
    LEAF,
    GOLD,
    TANGERINE,
    SIENNA,
}

// Do not use enums until they are supported in PAXEL
const GAME_STATE_PLAYING: u32 = 0;
const GAME_STATE_GAMEOVER: u32 = 1;
const GAME_STATE_WON: u32 = 2;

#[pax]
#[derive(PartialEq)]
pub enum GameState {
    #[default]
    Playing = 0,
    GameOver = 1,
    Won = 2,
}

impl Theme {
    pub fn as_color(&self) -> Color {
        match self {
            Self::CERULEAN => Color::from_hex("5390d9"),
            Self::CURRANT => Color::from_hex("2d3047"),
            Self::MARINE => Color::from_hex("264653"),
            Self::LILAC => Color::from_hex("6d597a"),
            Self::TEAL => Color::from_hex("2a9d8f"),
            Self::LEAF => Color::from_hex("8ab17d"),
            Self::GOLD => Color::from_hex("e9c46a"),
            Self::TANGERINE => Color::from_hex("f4a261"),
            Self::SIENNA => Color::from_hex("e76f51"),
        }
    }
}

impl Paddle {
    fn spawn(width: f64, height: f64) -> Self {
        Self {
            x: width * 0.5 - INITIAL_PADDLE_WIDTH / 2.0,
            y: height - PADDLE_HEIGHT - PADDLE_ELEVATION,
            width: INITIAL_PADDLE_WIDTH,
            height: PADDLE_HEIGHT,
            fill: Theme::GOLD.as_color(),
        }
    }
}

impl Ball {
    pub fn get_speed(&self) -> f64 {
        (self.dx.powi(2) + self.dy.powi(2)).sqrt()
    }

    fn spawn(width: f64, height: f64, upwards: bool) -> Self {
        let mut rng = rand::thread_rng();
        let mut angle = rng.gen_range(PI/4.0..3.0*PI/4.0);
        if upwards {
            angle *= -1.0
        }
        let dx = INITIAL_BALL_SPEED * angle.cos();
        let mut dy = INITIAL_BALL_SPEED * angle.sin();
        dy = dy.signum() * dy.abs().max(MIN_BALL_DY);
        Self {
            x: width / 2.0,
            y: height / 2.0,
            dx: dx,
            dy: dy,
            radius: BALL_SIZE / 2.0,
            visible: true,
            fill: Theme::GOLD.as_color(),
            trigger: None,
        }
    }

    /* Convenience debugging function: instead of changing velocity we change the color... */
    pub fn randomize_color(&mut self) {
        let mut rng = rand::thread_rng();
        let r = rng.gen_range(0.0..1.0);
        let g = rng.gen_range(0.0..1.0);
        let b = rng.gen_range(0.0..1.0);
        self.fill = Color::from_rgba_0_1([r, g, b, 1.0]);
    }

    pub fn randomize_velocity(&mut self) {
        self.randomize_color(); // convenient for debugging...
        let mut rng = rand::thread_rng();
        let mut theta = 0.0;
        let mut sign = 1.0;
        let (dx, dy) = loop {
            // Sample a rotation within some range for angular change,
            // only then determine the sign of the change separately
            theta = rng.gen_range(RANDOMIZE_VELOCITY_MIN..RANDOMIZE_VELOCITY_MAX);
            sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            theta *= sign;

            let dx = self.dx * theta.cos() - self.dy * theta.sin();
            let dy = self.dx * theta.sin() + self.dy * theta.cos();

            // Reject candidate rotation that change the direction of motion
            // along the y-axis: it would be too jarring to reverse momentum
            if dy.signum() != self.dy.signum() {
                continue;
            }

            if dy.abs() >= MIN_BALL_DY {
                break (dx, dy)
            }
        };
        self.dx = dx;
        self.dy = dy;
    }
}

impl BreakoutGame {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        // Immutable state
        self.brick_width.set(BRICK_WIDTH);
        self.brick_height.set(BRICK_HEIGHT);
        self.background_fill.set(Theme::CURRANT.as_color());
        self.debug_markers.set(SHOW_DEBUG_MARKERS);

        let (width, height) = ctx.bounds_parent.get();
        self.reset_game(width, height);
    }

    fn reset_game(&mut self, width: f64, height: f64) {
        self.paddle.set(Paddle::spawn(width, height));
        self.balls.set(vec![Ball::spawn(width, height, false)]);
        self.init_bricks(width, height);
        self.game_state.set(GameState::Playing as u32);
        self.score.set(0);
    }

    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        if self.game_state.get() != GameState::Playing as u32 {
            return;
        }

        let (width, height) = ctx.bounds_parent.get();
        let current_frame = ctx.frames_elapsed.get();

        let mut paddle = self.paddle.get();
        let mut balls = self.balls.get();
        let mut bricks = self.bricks.get();
        let mut new_balls = vec![];
        let mut rng = rand::thread_rng();

        // Update ball positions and handle collisions
        for ball in &mut balls {            
            match &mut ball.trigger {  // Randomize velocity event?
                None => {},
                Some(trigger) => {
                    if trigger.check(current_frame) {
                        ball.randomize_velocity();
                    }
                },
            }
            
            let mut nx = ball.x + ball.dx;  // next x
            let mut ny = ball.y + ball.dy;  // next y
            
            if nx - ball.radius <= 0.0 {
                ball.dx = -ball.dx;        // left wall bounce
                nx = ball.radius;          // prevent interpenetration
            } else if nx + ball.radius >= width {
                ball.dx = -ball.dx;        // right wall bounce
                nx = width - ball.radius;  // prevent interpenetration
            }

            if ny - ball.radius <= 0.0 {
                ball.dy = -ball.dy;        // top wall bounce
                ny = ball.radius;          // prevent interpenetration
            } else if ny + ball.radius >= height {
                ball.visible = false;      // flag for removal
            }
    
            // Check for paddle collision:
            // Find closest point C := (cx, cy) on rectangle to circle center,
            // then compare distance from C to ball center to the ball radius:
            let cx = nx.min(paddle.x + paddle.width).max(paddle.x);
            let cy = ny.min(paddle.y + paddle.height).max(paddle.y);
            let dx = paddle.width * PADDLE_REVERSE_BOUNCE_PORTION;

            // TODO: this does not prevent "tunnelling" at high-enough speeds
            if (nx - cx).powi(2) + (ny - cy).powi(2) <= ball.radius.powi(2) {
                if paddle.x + dx <= nx && nx <= paddle.x + paddle.width - dx {
                    ball.dy = -ball.dy;    // normal bounce on the long side
                    if ball.y <= paddle.y {
                        // prevent interpenetration from above
                        ny = paddle.y - ball.radius;
                    } else {
                        // prevent interpenetration from below
                        ny = paddle.y + paddle.height + ball.radius;
                    }
                /*
                } else if paddle.x <= nx && nx <= paddle.x + dx {
                    ball.dx = -ball.dx;    // weird bounce on the short side
                    ball.dy = -ball.dy;    // weird bounce on the short side
                } else if paddle.x + paddle.width - dx <= nx && nx <= paddle.x + paddle.width {
                    ball.dx = -ball.dx;    // weird bounce on the short side
                    ball.dy = -ball.dy;    // weird bounce on the short side
                 */
                } else {
                    ball.dx = -ball.dx;    // weird bounce on the short side
                    ball.dy = -ball.dy;    // weird bounce on the short side
                }
            }

            // Check for brick collisions
            for brick in &mut bricks {
                if !brick.visible {
                    continue  // bricks can only be destroyed once
                }

                if nx >= brick.x && nx <= brick.x + BRICK_WIDTH &&
                   ny >= brick.y && ny <= brick.y + BRICK_HEIGHT {
                    brick.visible = false;
                    ball.dy = -ball.dy;
                    let score = self.score.get() + 1;
                    self.score.set(score);
    
                    match brick.kind {
                        BrickType::SPAWN => {
                            new_balls.push(Ball::spawn(width, height, true));
                        },
                        BrickType::ACCELERATE => { // Red brick: increase ball speed
                            ball.dx *= BALL_SPEED_MULTIPLIER;
                            ball.dy *= BALL_SPEED_MULTIPLIER;
                            ball.fill = brick.fill.clone();
                        },
                        BrickType::PERTURB => { // Green brick: randomize ball velocity
                            // Compute the event occurrence rate from the intended average
                            // number of frames between each event in a Poisson process:
                            let interevent = rng.gen_range(INTEREVENT_INF..INTEREVENT_SUP);
                            let rate = 1.0 / interevent as f64 / APPROXIMATE_FPS as f64;
                            ball.trigger = Some(ExpTrigger::new(rate, current_frame));
                            ball.fill = brick.fill.clone();
                        },
                        BrickType::LONGER => {
                            paddle.width += PADDLE_WIDTH_INCREMENT;
                        },
                        BrickType::SHORTER => {
                            paddle.width -= PADDLE_WIDTH_INCREMENT;
                            paddle.width = paddle.width.max(MIN_PADDLE_WIDTH);
                        },
                        BrickType::NEUTRAL => {}
                    }
                    break;
                }
            }

            // Finally, we can commit to the ball's next position
            ball.x = nx;
            ball.y = ny;
        }
    
        balls.append(&mut new_balls);
        balls.retain(|ball| ball.visible);
        bricks.retain(|brick| brick.visible);
    
        if bricks.is_empty() {
            self.game_state.set(GameState::Won as u32);
        } else if balls.is_empty() {
            self.game_state.set(GameState::GameOver as u32);
        }
    
        self.paddle.set(paddle);
        self.balls.set(balls);
        self.bricks.set(bricks);
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        if self.game_state.get() == GameState::Playing as u32 {
            let mut paddle = self.paddle.get();
            let (width, _) = ctx.bounds_parent.get();
            let new_x = args.mouse.x - paddle.width / 2.0;
            paddle.x = new_x.clamp(0.0, width - paddle.width);
            self.paddle.set(paddle);
        }
    }

    pub fn handle_click(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if self.game_state.get() != GameState::Playing as u32 {
            let (width, height) = ctx.bounds_parent.get();
            self.reset_game(width, height);
        }
    }

    fn init_bricks(&mut self, width: f64, height: f64) {
        let start_x = (width - BRICK_COLS as f64 * BRICK_WIDTH) / 2.0;
        let start_y = height * 0.1;
        let mut rng = rand::thread_rng();
        let mut bricks = Vec::new();

        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                /* 
                 * Populate bricks and spawn each kind of brick randomly:
                 * - 50% chance for normal bricks
                 * - 25% chance for positive-effect bricks
                 * - 25% chance for negative-effect bricks
                 */
                let (kind, fill) = match rng.gen_range(0..100) {
                    0..=25 => {
                        match rng.gen_range(0..100) {
                            0..=50 => (BrickType::LONGER, Theme::LEAF.as_color()),
                            _ => (BrickType::SPAWN, Theme::CERULEAN.as_color()),
                        }
                    },
                    26..=50 => {
                        match rng.gen_range(0..100) {
                            0..=40 => (BrickType::ACCELERATE, Theme::TANGERINE.as_color()),
                            41..=80 => (BrickType::PERTURB, Theme::LILAC.as_color()),
                            _ => (BrickType::SHORTER, Theme::SIENNA.as_color()),
                        }
                    },
                    _ => (BrickType::NEUTRAL, Theme::GOLD.as_color()),
                };

                bricks.push(Brick {
                    x: start_x + col as f64 * BRICK_WIDTH,
                    y: start_y + row as f64 * BRICK_HEIGHT,
                    visible: true,
                    kind: kind,
                    fill: fill,
                });
            }
        }

        self.bricks.set(bricks);
    }
}