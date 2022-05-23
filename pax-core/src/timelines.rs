use pax_runtime_api::{Property, Tweenable};

pub struct PropertyTimeline {
    pub id: String,
    pub starting_value: Box<dyn Property<f64>>,
    pub timeline_segments: Vec<TimelineSegment>,
    pub cached_evaluated_value: f64,
    pub is_fresh: bool,
}

impl Property<f64> for PropertyTimeline {
    fn get(&self) -> &f64 {
        &self.cached_evaluated_value
    }

    fn _get_vtable_id(&self) -> Option<u64> {
        Some(self.id.as_str())
    }

    fn set(&mut self, value: f64) {
        self.cached_evaluated_value = value;
    }

    fn is_fresh(&self) -> bool {
        self.is_fresh
    }

    fn mark_fresh(&mut self) {
        self.is_fresh = true;
    }
}


pub struct TimelineSegment {
    pub curve_in: Box<dyn Tweenable>,
    pub ending_value: Box<dyn Property<f64>>,
    pub ending_frame_inclusive: usize,
}



//
// pub struct PropertyTimeline {
//     pub starting_value: Box<dyn Property<f64>>,
//     pub timeline_segments: Vec<TimelineSegment>,
//     pub cached_evaluated_value: f64,
// }

//TODO: create an Interpolatable trait that allows us
//      to ease between values beyond f64 (e.g. a discrete interpolator
// //      for ints or vecs; an interpolator for Colors)
// impl ComputableProperty for PropertyTimeline {
//
//     fn compute_in_place(&mut self, rtc: &RenderTreeContext) {
//         let timeline_playhead_position = rtc.timeline_playhead_position;
//         let mut starting_frame : usize = 0;
//         let mut starting_value = self.starting_value.get();
//         let mut segments_iter = self.timeline_segments.iter().peekable();
//         let mut active_segment = match segments_iter.next() {
//             Some(seg) => seg,
//             None => {panic!("Timeline properties must define at least one TimelineSegment.")}
//         };
//
//         //Scan through our list of timeline segments to find our active segment
//         //TODO:  this lookup could be optimized to constant-time with something like
//         //       a tree-map, or a "'''hashmap''' with ranges-of-frames for keys => pointers-to-segments for values"
//         while timeline_playhead_position > active_segment.ending_frame_inclusive
//             && segments_iter.peek().is_some()
//         {
//             starting_frame = active_segment.ending_frame_inclusive;
//             starting_value = active_segment.ending_value.get();
//             active_segment = segments_iter.next().unwrap();
//         };
//
//         // Determine how far along the playhead is between starting_frame and
//         // the current segment's ending_frame.  That ratio [0,100%] (capped)
//         // is the number to pass into our easing curve
//         let progress = (
//             (timeline_playhead_position - starting_frame) as f64
//             /
//             (active_segment.ending_frame_inclusive - starting_frame) as f64
//         ).min(1.0); //cap at 1.0 to satisfy domain expectations of easing functions [0,1]
//
//         let progress_eased = active_segment.curve_in.map(progress);
//
//         //the computed value is a function of the magnitude of difference
//         //between val_last and val_next.  Keep in mind that progress_eased is NOT
//         //bound to [0,1], because some easing curves can "hyperextend" their
//         //interpolation, e.g. a standard elastic curve.  Such hyperextension, too,
//         //is a function of the magnitude of the difference between val_last and val_next.
//         let ending_value = active_segment.ending_value.get();
//
//         self.cached_evaluated_value = starting_value + (progress_eased * (ending_value - starting_value));
//     }
//
//
// }
