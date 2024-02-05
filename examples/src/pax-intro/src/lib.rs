#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Fidget {
    pub tiles: Property<Vec<Tile>>,
    pub tiles_mobile: Property<Vec<Tile>>,
    pub ticks: Property<usize>,
    pub last_pointer_x: Property<f64>,
    pub last_pointer_y: Property<f64>,
    pub is_mobile: Property<bool>,
}

pub fn is_mobile(viewport_width: f64, _viewport_height: f64) -> bool {
    viewport_width < 500.0
}

const LAYOUT_WIDTH : usize = 6;
const LAYOUT_HEIGHT : usize = 5;
const LAYOUT_WIDTH_MOBILE : usize = 6;
const LAYOUT_HEIGHT_MOBILE : usize = 5;

impl Fidget {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.tiles.set(vec![Tile::default(); LAYOUT_WIDTH * LAYOUT_HEIGHT]);
        self.tiles_mobile.set(vec![Tile::default(); LAYOUT_WIDTH_MOBILE * LAYOUT_HEIGHT_MOBILE]);
    }

    pub fn advance_tiles_in_place(&mut self) {
        //TODO: satisfy constraints:

        // Rules:
        // (Particles are arranged previously in a grid, with `x = x_home` and `y = y_home` describing their centers and `w` and `h` describing their entire width & height (2 * extents)
        // Each tile must end each tick within a circle described by the radius MAX_DISTANCE (enforce this at end of computation)
        // In the absence of other active forces, each tile wants to return to its home, `x_home`, `y_home`.
        // Each tick, in the absence of other stimuli, each tile reduces its velocity by FRICTION_COEFFICIENT
        // When a pointer (tracked by self.last_pointer_x, self.last_pointer_y) enters a tile, nothing changes — but when a pointer _leaves_ a tile's bounding box, the tile "sticks" to the mouse and updates itself to follow the pointer (strictly speaking, to continue _containing_ the pointer) — until the limit MAX_DISTANCE is reached.  After that time, the tile jumps back to its home (x_home, y_home) via a simple spring mechanism
        let new_tiles = if *self.is_mobile.get() {
             self.tiles_mobile.get().clone()
        } else {
            self.tiles.get().clone()
        };

        if *self.is_mobile.get() {
            self.tiles_mobile.set(new_tiles);
        } else {
            self.tiles.set(new_tiles);
        }
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let is_mobile = is_mobile(ctx.bounds_parent.0, ctx.bounds_parent.1);
        self.is_mobile.set(is_mobile);
        self.advance_tiles_in_place();
        self.ticks.set(self.ticks.get() + 1);
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _args: ArgsClick) {}

    pub fn handle_touch_move(&mut self, _ctx: &NodeContext, args: ArgsTouchMove) {
        self.last_pointer_x.set(args.touches[0].x);
        self.last_pointer_y.set(args.touches[0].y);
    }

    pub fn handle_mouse_move(&mut self, _ctx: &NodeContext, args: ArgsMouseMove) {
        self.last_pointer_x.set(args.mouse.x);
        self.last_pointer_y.set(args.mouse.y);
    }
}


#[pax]
#[custom(Imports)]
pub struct Tile {
    pub x: Size,
    pub x_home: Size,
    pub x_prev: Size,
    pub x_vel: f64,
    pub y: Size,
    pub y_home: Size,
    pub y_prev: Size,
    pub y_vel: f64,
    pub w: Size,
    pub h: Size,
}


