pub mod coordinate_spaces {

    use pax_engine::math;

    pub struct Glass;

    impl math::Space for Glass {}

    pub struct World;

    impl math::Space for World {}
}
