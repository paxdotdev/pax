#[allow(unused)]
use crate::*;
use pax_engine::api::{Event, Wheel};
use pax_engine::api::{Property, Size};
use pax_engine::*;
use pax_runtime::api::{NodeContext, TouchEnd, TouchMove, TouchStart, OS};
use std::cell::RefCell;
use std::collections::HashMap;

// a carousel container
#[pax]
#[file("carousel.pax")]
pub struct Carousel {
    // > 0.0 means objects further away are offset upwards, with the furthest
    // away object being a maximum of this value above the bounds of the carousel
    pub perspective_offset: Property<f64>,
    // how much of the carousel width should each object take up?
    pub object_width: Property<Size>,
    // 0.5 makes the objects furthest away be half the size of the one in front
    pub size_scaling_factor: Property<f64>,
    // time for a transition (seconds)
    pub transition_time: Property<f64>,
}

#[pax]
pub struct CarouselObjectData {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub index: u32,
}

impl Carousel {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children_count = ctx.slot_children_count.clone();
        let deps = [slot_children_count.untyped()];
        self._slot_children_count
            .replace_with(Property::computed(move || slot_children_count.get(), &deps));
        let scroll_params = match ctx.os {
            OS::Android => PlatformSpecificScrollParams {
                deacceleration: 0.02,
                damping: 0.03,
                fling: true,
            },
            OS::IPhone => PlatformSpecificScrollParams {
                deacceleration: 0.00,
                damping: 0.04,
                fling: false,
            },
            // just choose some hopefully sane default
            OS::Windows | OS::Mac | OS::Linux | OS::Unknown => PlatformSpecificScrollParams {
                deacceleration: 0.01,
                damping: 0.04,
                fling: false,
            },
        };
        self._platform_params.set(scroll_params);
    }

    pub fn update(&mut self, ctx: &NodeContext) {}
}
