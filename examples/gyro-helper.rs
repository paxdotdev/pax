fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(gyro_helper)?.run()
}
