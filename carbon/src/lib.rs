

pub use piet_web::WebRenderContext;
pub use piet::Error;











pub struct CarbonEngine {
    // tick_and_render: fn(&mut Context) -> Result<(), Error>
}

pub fn get_engine() -> CarbonEngine {
    return CarbonEngine::new();
}

impl CarbonEngine {
    fn new() -> Self {
        CarbonEngine {}
    }

    pub fn tick_and_render (&self, mut _ctx: WebRenderContext) -> Result<(), Error> {
        Ok(())
    }
}

