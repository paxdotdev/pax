#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("marionette_rig.pax")]
pub struct MarionetteRig {
    pub torso_rotate: Property<f64>,
    pub torso_shift_y: Property<f64>,
    pub head_rotate: Property<f64>,
    pub head_shift_y: Property<f64>,
    pub left_arm_t: Property<f64>,
    pub right_arm_t: Property<f64>,
    pub left_leg_t: Property<f64>,
    pub right_leg_t: Property<f64>,

    pub left_upper_arm_angle: Property<f64>,
    pub left_elbow_t: Property<f64>,
    pub left_forearm_angle: Property<f64>,
    pub left_wrist_t: Property<f64>,
    pub left_hand_angle: Property<f64>,

    pub right_upper_arm_angle: Property<f64>,
    pub right_elbow_t: Property<f64>,
    pub right_forearm_angle: Property<f64>,
    pub right_wrist_t: Property<f64>,
    pub right_hand_angle: Property<f64>,

    pub left_thigh_angle: Property<f64>,
    pub left_knee_t: Property<f64>,
    pub left_shin_angle: Property<f64>,
    pub left_ankle_t: Property<f64>,
    pub left_foot_angle: Property<f64>,

    pub right_thigh_angle: Property<f64>,
    pub right_knee_t: Property<f64>,
    pub right_shin_angle: Property<f64>,
    pub right_ankle_t: Property<f64>,
    pub right_foot_angle: Property<f64>,
}
