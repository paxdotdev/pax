//! Scoped, opt-in CPU attribution. No scene data or timestamps survive a service.

use super::surface_work::{PreparationDiagnostics, PreparationStageTiming};
use std::cell::RefCell;
use web_time::Instant;

#[derive(Clone, Copy)]
pub(super) enum Stage {
    Selection,
    Capture,
    Replay,
    ImageReplay,
    Encode,
    Submit,
    Cleanup,
    Present,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Selection => "selection",
            Self::Capture => "capture",
            Self::Replay => "replay",
            Self::ImageReplay => "image_replay",
            Self::Encode => "encode",
            Self::Submit => "submit",
            Self::Cleanup => "cleanup",
            Self::Present => "present",
        }
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<PreparationDiagnostics>> = const { RefCell::new(None) };
}

pub(super) struct Capture(bool);
impl Capture {
    pub fn new(enabled: bool) -> Self {
        if enabled {
            ACTIVE.with(|active| {
                let mut active = active.borrow_mut();
                assert!(active.is_none(), "nested surface diagnostic capture");
                *active = Some(PreparationDiagnostics::default());
            });
        }
        Self(enabled)
    }

    pub fn finish(self) -> Option<PreparationDiagnostics> {
        self.0
            .then(|| ACTIVE.with(|active| active.borrow_mut().take()))
            .flatten()
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if self.0 {
            ACTIVE.with(|active| active.borrow_mut().take());
        }
    }
}

pub(super) struct Span(Stage, Option<Instant>);
impl Span {
    pub fn new(stage: Stage) -> Self {
        Self(
            stage,
            ACTIVE
                .with(|active| active.borrow().is_some())
                .then(Instant::now),
        )
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.1 {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            ACTIVE.with(|active| {
                if let Some(active) = active.borrow_mut().as_mut() {
                    let sample = active
                        .stages
                        .entry(self.0.name())
                        .or_insert_with(PreparationStageTiming::default);
                    sample.count += 1;
                    sample.total_ms += ms;
                    sample.max_ms = sample.max_ms.max(ms);
                }
            });
        }
    }
}

pub(super) fn surfaces(targeted: usize, additional: usize, uploads: usize, bytes: usize) {
    ACTIVE.with(|active| {
        if let Some(active) = active.borrow_mut().as_mut() {
            active.targeted_surfaces += targeted;
            active.additional_surfaces += additional;
            active.image_uploads += uploads;
            active.image_upload_bytes += bytes;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_is_opt_in_bounded_and_does_not_leak_between_services() {
        let off = Capture::new(false);
        drop(Span::new(Stage::Capture));
        assert!(off.finish().is_none());
        let on = Capture::new(true);
        for _ in 0..100 {
            drop(Span::new(Stage::Replay));
        }
        surfaces(1, 2, 3, 4096);
        let report = on.finish().unwrap();
        assert_eq!(report.stages.len(), 1);
        assert_eq!(report.stages["replay"].count, 100);
        assert_eq!(report.additional_surfaces, 2);
        assert_eq!(report.image_upload_bytes, 4096);
        drop(Capture::new(true)); // Early return/unwind also clears the scope.
        assert!(Capture::new(true).finish().unwrap().stages.is_empty());
    }
}
