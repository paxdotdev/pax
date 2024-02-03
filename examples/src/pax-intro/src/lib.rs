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
pub struct DynamicObject {
    pub ticks: Property<f64>,
    pub mouse_x: Property<f64>,
    pub mouse_y: Property<f64>,
    pub rects: Property<Vec<Rect>>,
    pub rects_bellow: Property<Vec<Rect>>,
}

#[pax]
#[custom(Imports)]
pub struct Rect {
    pub x: Size,
    pub y: Size,
    pub w: Size,
    pub h: Size,
}

const N_X: f64 = 10.0;
const N_Y: f64 = 10.0;
const LEN: usize = N_X as usize * N_Y as usize;

impl DynamicObject {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.rects.set(vec![Rect::default(); LEN]);
        self.rects_bellow.set(vec![Rect::default(); LEN]);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let offsets = [0.1, 0.5, 1.0, 0.2, 0.3, 0.9, 0.3, 0.8, 0.7, 0.6];
        let div_line = [-4, -1, -2, 0, 1, -1, 0, 3, 2, 4];
        let t = *self.ticks.get();
        self.ticks.set(t + 0.01);
        let (b_x, b_y) = ctx.bounds_self;
        let sp_ratio = 0.1;
        let sp = b_x * sp_ratio / (N_X + 1.0);
        let r_w = (b_x * (1.0 - sp_ratio)) / N_X;
        let r_h = (b_y - sp * (N_Y + 1.0)) / N_Y;
        for i_int in 0..N_X as usize {
            for j_int in 0..N_Y as usize {
                let i = i_int as f64;
                let j = j_int as f64;
                let wave_x = i.sin();
                let wave_y = j.sin();
                let mut x = sp
                    + i as f64 * (sp + r_w)
                    + smooth_sawtooth(i * 0.1 + offsets[j_int % offsets.len()] * 2.0 + t) * sp
                        / 2.0
                    + wave_x;
                let y = sp
                    + j as f64 * (sp + r_h)
                    + smooth_sawtooth(j * 0.1 + offsets[i_int % offsets.len()] + t * 0.73) * sp
                        / 2.0
                    + wave_y;

                let cent_offset_x = i - N_X / 2.0 + 1.0 - div_line[j_int] as f64;
                let cent_offset_y = j - N_Y / 2.0 + 1.0;
                let dir = if cent_offset_x > 0.0 { 1.0 } else { -1.0 };
                let delay = -cent_offset_x.abs() * 0.02 - (1.0 - cent_offset_y.abs() * 0.2) * 0.5;
                x += dir * smoothstep(2.0 + delay, 2.8 + delay, t) * 5.0 * r_w;
                let ind = i_int + j_int * N_X as usize;
                let rect = &mut self.rects.get_mut()[ind];
                rect.x = Size::Pixels(x.into());
                rect.y = Size::Pixels(y.into());
                rect.w = Size::Pixels((r_w).into());
                rect.h = Size::Pixels((r_h).into());
                let rect_b = &mut self.rects_bellow.get_mut()[ind];
                rect_b.x = Size::Pixels((x - sp).into());
                rect_b.y = Size::Pixels((y - sp).into());
                rect_b.w = Size::Pixels((r_w + 2.0 * sp).into());
                rect_b.h = Size::Pixels((r_h + 2.0 * sp).into());
            }
        }

        //hack to make repeat refresh
        if self.rects.get().len() <= LEN {
            self.rects.get_mut().push(Rect::default());
            self.rects_bellow.get_mut().push(Rect::default());
        } else {
            self.rects.get_mut().pop();
            self.rects_bellow.get_mut().pop();
        }
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _args: ArgsClick) {}

    pub fn mouse_move(&mut self, _ctx: &NodeContext, args: ArgsMouseMove) {
        self.mouse_x.set(args.mouse.x);
        self.mouse_y.set(args.mouse.y);
    }
}

fn smooth_sawtooth(t: f64) -> f64 {
    const N: f64 = 5.0;
    let p = |x: f64| x * (1.0 - x.powf(2.0 * N));
    p(t.rem_euclid(2.0) - 1.0)
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - t * 2.0)
}
