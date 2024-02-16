pub mod coordinate_spaces {

    use pax_engine::math;
    #[derive(Clone, Copy)]
    pub struct Window;

    impl math::Space for Window {}

    #[derive(Clone, Copy)]
    pub struct Glass;

    impl math::Space for Glass {}

    #[derive(Clone, Copy)]
    pub struct World;

    impl math::Space for World {}
}
