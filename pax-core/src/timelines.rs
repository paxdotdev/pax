use pax_runtime_api::{Property, Tweenable};

pub struct PropertyTimeline {
    pub id: String,
    pub starting_value: Box<dyn Property<f64>>,
    pub timeline_segments: Vec<TimelineSegment>,
    pub cached_evaluated_value: f64,
}

impl Property<f64> for PropertyTimeline {
    fn get(&self) -> &f64 {
        &self.cached_evaluated_value
    }

    fn _get_vtable_id(&self) -> Option<&str> {
        Some(self.id.as_str())
    }

    fn set(&mut self, value: f64) {
        self.cached_evaluated_value = value;
    }
}


pub struct TimelineSegment {
    pub curve_in: Box<dyn Tweenable>,
    pub ending_value: Box<dyn Property<f64>>,
    pub ending_frame_inclusive: usize,
}