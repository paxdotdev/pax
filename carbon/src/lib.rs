

use piet_web::WebRenderContext;
pub use piet::Error;

// Alias so that consumers need only import `carbon`
use WebRenderContext as Context;



pub struct Engine {
    tick_and_render: fn(&mut Context) -> Result<(), Error>
}



pub fn get_engine() -> Engine {
    return Engine::new();
}

