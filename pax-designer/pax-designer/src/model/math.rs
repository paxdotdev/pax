use pax_engine::math;

#[derive(Clone, Copy)]
pub struct Screen;

impl math::Space for Screen {}

#[derive(Clone, Copy)]
pub struct Glass;

impl math::Space for Glass {}

#[derive(Clone, Copy)]
pub struct World;

impl math::Space for World {}
