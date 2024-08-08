
use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
pub struct Animation {
    pub frame: ImageSource,
    pub frames: Vec<ImageSource>,
    pub running: bool,
    pub finished: bool,
    pub time: u64,

    //settable properties
    pub speed: u64,
    pub loops: bool,
}

impl Animation {
    pub fn new(frames: Vec<ImageSource>) -> Self {
        Self {
            frame: frames[0].clone(),
            frames,
            running: false,
            loops: false,
            finished: false,
            speed: 10,
            time: 0,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn tick(&mut self) {
        if self.running {
            self.time += 1;
            let frame_index = (self.time / self.speed) as usize;
            if frame_index < self.frames.len() {
                self.frame = self.frames[frame_index].clone();
                return;
            }
            if self.loops {
                self.start();
            } else {
                self.running = false;
                self.finished = true;
            }
        }
    }
}
