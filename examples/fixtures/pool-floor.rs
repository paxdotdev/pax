fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(pool_floor)?.run()
}
