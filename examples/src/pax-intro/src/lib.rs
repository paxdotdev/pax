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
const LAYOUT_WIDTH_MOBILE : usize = 3;
const LAYOUT_HEIGHT_MOBILE : usize = 4;
const MAX_DISTANCE: f64 = 100.0; 
const FRICTION_COEFFICIENT: f64 = 0.95;
const SPRING_CONSTANT: f64 = 0.1;

impl Fidget {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.tiles.set(vec![Tile::default(); LAYOUT_WIDTH * LAYOUT_HEIGHT]);
        self.tiles_mobile.set(vec![Tile::default(); LAYOUT_WIDTH_MOBILE * LAYOUT_HEIGHT_MOBILE]);

        self.set_tile_home_based_on_viewport(ctx.bounds_parent.0, ctx.bounds_parent.1);
    }

    pub fn set_tile_home_based_on_viewport(&mut self, viewport_width: f64, viewport_height: f64) {
        let mut new_tiles = if *self.is_mobile.get() {
            self.tiles_mobile.get().clone()
        }  else {
            self.tiles.get().clone()
        };

        // Calculate the horizontal and vertical spacing between tile centers
        let horizontal_spacing = viewport_width / LAYOUT_WIDTH_MOBILE as f64;
        let vertical_spacing = viewport_height / LAYOUT_HEIGHT_MOBILE as f64;

        let width = horizontal_spacing.min(vertical_spacing);
        let height = width;

        // Calculate the starting center position
        let start_x_center = horizontal_spacing / 2.0;
        let start_y_center = vertical_spacing / 2.0;

        // Iterate over the grid
        for i in 0..LAYOUT_WIDTH_MOBILE {
            for j in 0..LAYOUT_HEIGHT_MOBILE {
                // Calculate the index in the linear array from the grid position
                let index = j * LAYOUT_WIDTH_MOBILE + i;

                // Calculate the center position for each tile
                let x_center = start_x_center + i as f64 * horizontal_spacing;
                let y_center = start_y_center + j as f64 * vertical_spacing;

                // Update the home positions of the tile
                if let Some(tile) = new_tiles.get_mut(index) {
                    tile.x_home = x_center.into();
                    tile.y_home = y_center.into();
                    tile.w = width;
                    tile.h = height;
                }
            }
        }

        // Update the property with the modified tiles
        if *self.is_mobile.get() {
            self.tiles_mobile.set(new_tiles);
        } else {
            self.tiles.set(new_tiles);
        }
    }

    pub fn advance_tiles_in_place(&mut self) {
        let pointer_x = *self.last_pointer_x.get();
        let pointer_y = *self.last_pointer_y.get();

        let mut new_tiles = if *self.is_mobile.get() {
            self.tiles_mobile.get().clone()
        } else {
            self.tiles.get().clone()
        };

        for tile in new_tiles.iter_mut() {
            // Apply friction to velocity
            tile.x_vel *= FRICTION_COEFFICIENT;
            tile.y_vel *= FRICTION_COEFFICIENT;

            // Check if pointer is within the tile's bounding box
            if pointer_x >= tile.x - tile.w / 2.0 && pointer_x <= tile.x + tile.w / 2.0 &&
               pointer_y >= tile.y - tile.h / 2.0 && pointer_y <= tile.y + tile.h / 2.0 {
                // If within bounding box, stick to the pointer
                tile.x_vel = pointer_x - tile.x_prev;
                tile.y_vel = pointer_y - tile.y_prev;
            } else {
                // Apply spring force towards home position
                tile.x_vel += (tile.x_home - tile.x) * SPRING_CONSTANT;
                tile.y_vel += (tile.y_home - tile.y) * SPRING_CONSTANT;
            }

            // Update positions
            tile.x_prev = tile.x;
            tile.y_prev = tile.y;
            tile.x += tile.x_vel;
            tile.y += tile.y_vel;

            // Ensure tile remains within MAX_DISTANCE from its home
            let distance = ((tile.x - tile.x_home).powi(2) + (tile.y - tile.y_home).powi(2)).sqrt();
            if distance > MAX_DISTANCE {
                let scale = MAX_DISTANCE / distance;
                tile.x = tile.x_home + (tile.x - tile.x_home) * scale;
                tile.y = tile.y_home + (tile.y - tile.y_home) * scale;
            }
        }

        // Update the property with the modified tiles
        if *self.is_mobile.get() {
            self.tiles_mobile.set(new_tiles.clone());
        } else {
            self.tiles.set(new_tiles.clone());
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
    pub x: f64,
    pub x_home: f64,
    pub x_prev: f64,
    pub x_vel: f64,
    pub y: f64,
    pub y_home: f64,
    pub y_prev: f64,
    pub y_vel: f64,
    pub w: f64,
    pub h: f64,
}


