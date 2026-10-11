fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!(pick_up_sticks)?.run()
}
