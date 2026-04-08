#![allow(unused_imports)]

pub mod marionette_rig;
use marionette_rig::MarionetteRig;

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub selected_action: Property<u32>,
    pub master_playhead: Property<f64>,

    pub idle_torso_rotate: Property<f64>,
    pub idle_torso_shift_y: Property<f64>,
    pub idle_head_rotate: Property<f64>,
    pub idle_head_shift_y: Property<f64>,
    pub idle_left_arm_t: Property<f64>,
    pub idle_right_arm_t: Property<f64>,
    pub idle_left_leg_t: Property<f64>,
    pub idle_right_leg_t: Property<f64>,

    pub wave_torso_rotate: Property<f64>,
    pub wave_torso_shift_y: Property<f64>,
    pub wave_head_rotate: Property<f64>,
    pub wave_head_shift_y: Property<f64>,
    pub wave_left_arm_t: Property<f64>,
    pub wave_right_arm_t: Property<f64>,
    pub wave_left_leg_t: Property<f64>,
    pub wave_right_leg_t: Property<f64>,

    pub bow_torso_rotate: Property<f64>,
    pub bow_torso_shift_y: Property<f64>,
    pub bow_head_rotate: Property<f64>,
    pub bow_head_shift_y: Property<f64>,
    pub bow_left_arm_t: Property<f64>,
    pub bow_right_arm_t: Property<f64>,
    pub bow_left_leg_t: Property<f64>,
    pub bow_right_leg_t: Property<f64>,

    pub hero_torso_rotate: Property<f64>,
    pub hero_torso_shift_y: Property<f64>,
    pub hero_head_rotate: Property<f64>,
    pub hero_head_shift_y: Property<f64>,
    pub hero_left_arm_t: Property<f64>,
    pub hero_right_arm_t: Property<f64>,
    pub hero_left_leg_t: Property<f64>,
    pub hero_right_leg_t: Property<f64>,

    pub sneak_torso_rotate: Property<f64>,
    pub sneak_torso_shift_y: Property<f64>,
    pub sneak_head_rotate: Property<f64>,
    pub sneak_head_shift_y: Property<f64>,
    pub sneak_left_arm_t: Property<f64>,
    pub sneak_right_arm_t: Property<f64>,
    pub sneak_left_leg_t: Property<f64>,
    pub sneak_right_leg_t: Property<f64>,

    pub kick_torso_rotate: Property<f64>,
    pub kick_torso_shift_y: Property<f64>,
    pub kick_head_rotate: Property<f64>,
    pub kick_head_shift_y: Property<f64>,
    pub kick_left_arm_t: Property<f64>,
    pub kick_right_arm_t: Property<f64>,
    pub kick_left_leg_t: Property<f64>,
    pub kick_right_leg_t: Property<f64>,
}
