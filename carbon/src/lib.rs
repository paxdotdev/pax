

pub use piet_web::WebRenderContext;
pub use piet::{Error, Color};
use piet::RenderContext;


pub struct CarbonEngine {
    // tick_and_render: fn(&mut Context) -> Result<(), Error>
    frames_elapsed: u32
}

pub fn get_engine() -> CarbonEngine {
    return CarbonEngine::new();
}

impl CarbonEngine {
    fn new() -> Self {
        CarbonEngine {
            frames_elapsed: 0
        }
    }

    pub fn tick_and_render (&mut self, rc: &mut WebRenderContext) -> Result<(), Error> {
        // let red = Color::rgb8(255,0,0);
        const speed : f64 = 2.0;

        let hue = (((self.frames_elapsed + 1) as f64 * speed) as i64 % 360) as f64;
        let current_color = Color::hlc(hue, 75.0, 127.0);
        rc.clear(current_color);
        //TODO: add hello-world rendering logic here; HSL-rotation?
        self.frames_elapsed = self.frames_elapsed + 1;
        Ok(())
    }
}

