use pax_kit::*;
use std::cell::RefCell;
mod notes;
thread_local! {
    static NOTES: RefCell<notes::Loader> = RefCell::new(notes::Loader::default());
}

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}

impl Example {
    pub fn on_tick(&mut self, _ctx: &NodeContext) {
        // One application-level completion drain, never geometry polling per card.
        let deliveries = NOTES.with(|loader| loader.borrow_mut().poll());
        for (lease, callback, result) in deliveries {
            callback(lease, result);
        }
    }

    pub fn on_unmount(&mut self, _ctx: &NodeContext) {
        NOTES.with(|loader| *loader.borrow_mut() = notes::Loader::default());
    }
}

#[pax]
#[file("card.pax")]
pub struct ProximityCard {
    pub index: Property<usize>,
    pub playhead: Property<f64>,
    pub entries: Property<usize>,
    pub samples: Property<usize>,
    pub visible_pixels: Property<usize>,
    pub departing: Property<bool>,
    pub leased: Property<bool>,
    pub request_lease: Property<u64>,
    pub detail: Property<String>,
    pub stage: Property<String>,
}

impl ProximityCard {
    pub fn enter(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityEnter>) {
        self.entries.set(self.entries.get() + 1);
        self.leased.set(true);
        self.stage.set("NEARBY".into());
        self.load_detail();
    }

    fn load_detail(&mut self) {
        NOTES.with(|loader| loader.borrow_mut().cache.release(self.request_lease.get()));
        let expected = self.index.get();
        let index = self.index.clone();
        let lease = self.request_lease.clone();
        let nearby = self.leased.clone();
        let detail = self.detail.clone();
        if detail.get().is_empty() {
            detail.set("Loading field note…".into());
        }
        let request = NOTES.with(|loader| {
            loader
                .borrow_mut()
                .acquire(expected / 6, move |token, result| {
                    // A completion may arrive after exit, re-entry, or data identity change.
                    if nearby.get() && lease.get() == token && index.get() == expected {
                        detail.set(match result {
                            Ok(notes) => notes[expected % 6].clone(),
                            Err(_) => "Field note unavailable.".into(),
                        });
                    }
                })
        });
        self.request_lease.set(request);
    }

    fn release_detail(&mut self) {
        NOTES.with(|loader| loader.borrow_mut().cache.release(self.request_lease.get()));
        self.request_lease.set(0);
        self.leased.set(false);
    }

    fn reveal(&mut self) {
        self.departing.set(false);
        self.stage.set("VISIBLE".into());
        self.playhead.cancel_transitions();
        self.playhead.ease_to(
            100.0,
            Duration::Milliseconds(420.into()),
            EasingCurve::Linear,
        );
    }

    pub fn change(&mut self, _ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.samples.set(self.samples.get() + 1);
        let visible = event
            .current
            .viewport_intersection
            .map_or(0.0, |r| r.height());
        let previous = event
            .previous
            .and_then(|p| p.viewport_intersection)
            .map_or(0.0, |r| r.height());
        self.visible_pixels.set(visible.ceil() as usize);
        if visible > 0.0 && (previous == 0.0 || (self.departing.get() && visible > previous)) {
            self.reveal();
        } else if visible <= 40.0 && visible < previous && !self.departing.get() {
            self.departing.set(true);
            self.stage.set("GOODBYE".into());
            self.playhead.cancel_transitions();
            self.playhead
                .ease_to(0.0, Duration::Milliseconds(180.into()), EasingCurve::Linear);
        }
    }

    pub fn exit(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityExit>) {
        self.release_detail();
        self.departing.set(false);
        self.playhead.cancel_transitions();
        self.playhead.set(0.0);
        self.stage.set("COLD".into());
    }

    pub fn on_unmount(&mut self, _ctx: &NodeContext) {
        // Unmount has its own cleanup path; it does not synthesize proximity exit.
        self.release_detail();
    }
}
