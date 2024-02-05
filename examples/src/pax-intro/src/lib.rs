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

    pub last_bounds_x: Property<f64>,
    pub last_bounds_y: Property<f64>,

    pub is_mobile: Property<bool>,
    
}


pub fn is_mobile(viewport_width: f64, _viewport_height: f64) -> bool {
    viewport_width < 500.0
}

impl Fidget {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        // self.tiles.set(vec![Tile::default(); LEN]);
    }


    pub fn advance_tiles(&mut self, active_tiles: &Vec<Tile>) -> Vec<Tile> {
        //TODO: satisfy constraints:
        //  - Each particle (`Tile`) has a home, (x_home, y_home).  These home points are evenly distributed across a grid, as if by two Stackers (but computed manually) 
        //  - Tiles repel each other, disallowing overlapping at their bounds.  We can assume a particle will never be more than one neighor's-distance away from its home,
        //    so we only need to check repulsion vs. our 8 immediate neighbors
        //  - Particles allow pointer to enter their bounds without disturbance, but "stick" for a while as the pointer leaves, following the pointer to a maximum `stickiness extent`, and "snapping back" after the cursor moves beyond that extent.
        //  - In the absence of pointer that is constraining a particle's location, particles want to return to their centers on a spring
        //Tune the system such that bounce-back will cause ripple effects à la wake behind the user's mouse / cursor.

    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let is_mobile = is_mobile(ctx.bounds_parent.0, ctx.bounds_parent.1);
        self.is_mobile.set(is_mobile);
        
        if is_mobile {
            //operate on self.tiles_mobiles
            self.advance_tiles(&self.tiles_mobile.get());
        } else {
            //operate on self.tiles
            self.advance_tiles(&self.tiles.get());
        }

        self.ticks.set(self.ticks.get() + 1);
        // self.tiles.set(new_tiles);
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
    pub y: Size,
    pub y_home: Size,
    pub y_prev: Size,
    pub w: Size,
    pub h: Size,
}


