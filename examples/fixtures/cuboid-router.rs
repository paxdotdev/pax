fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(cuboid_router)?.run()
}
