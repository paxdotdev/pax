//! Standard loading experience, authored with the same context as custom loaders.

#[allow(unused_imports)] // Component paths used by the Pax template compiler.
use crate::{Button, Group, LogoBackgroundMode, Rectangle, StaticPaxLogo, Text};
use pax_engine::api::*;
use pax_engine::*;

/// A still Pax logo and an unlabeled progress bar on a black background.
///
/// Requires a loading scope supplying `@loading`. The first 67% of the bar
/// reports decoded download progress. With an unknown download size, or during
/// preparation, the same fill advances monotonically toward a reserved limit;
/// this is visual activity, not a measured initialization percentage.
/// A 250 ms exit transition fills the remaining bar while fading the screen
/// over the prepared application. Retrying starts a new progress cycle.
/// Failure/cancellation offers retry only when supported.
/// The optional `@replay` custom event is intended for loader previews; clicking
/// the logo never restarts an application download on its own.
///
/// Select this screen on Web with `[package.metadata.pax] loader = true`.
/// Startup packaging is experimental (PAX-962). An ordinary component
/// invocation outside a loading scope does not create a startup coordinator.
#[pax]
#[engine_import_path("pax_engine")]
#[file("loading_screen/default_loading_screen.pax")]
pub struct DefaultLoadingScreen {
    /// Visual fill before the exit timeline; maintained by the loading screen.
    /// This is not an overall measured loading percentage.
    pub display_fraction: Property<f64>,
}

impl DefaultLoadingScreen {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let Some(loading) = ctx.loading() else { return };
        let state = loading.observe();
        let millis = ctx.elapsed_millis.clone();
        let fraction = self.display_fraction.local();
        let dependencies = [state.untyped(), millis.untyped()];
        let presentation = std::cell::Cell::new(LoadingBarProgress::default());
        let update = move || {
            let Some(state) = state.get() else { return };
            let mut progress = presentation.get();
            let next = progress.advance(&state, millis.get());
            presentation.set(progress);
            if fraction.get() != next {
                fraction.set(next);
            }
        };
        update();
        ctx.subscribe(&dependencies, update);
    }

    pub fn replay(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        // A preview can bind this action; normal startup has no replay binding.
        let _ = ctx.dispatch_event("replay");
    }

    pub fn retry(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        if let Some(loading) = ctx.loading() {
            loading.retry();
        }
    }
}

// Keep the visual estimate local to this component. Readiness and byte progress
// remain authoritative host state; only the authored exit may finish the bar.
#[derive(Clone, Copy, Default)]
struct LoadingBarProgress {
    phase: Option<LoadingPhase>,
    phase_started_ms: u64,
    fraction: f64,
}

impl LoadingBarProgress {
    fn advance(&mut self, state: &LoadingState, millis: u64) -> f64 {
        const DOWNLOAD_SHARE: f64 = 0.67;
        const ACTIVITY_HEADROOM: f64 = 0.05;
        if self.phase != Some(state.phase) {
            self.phase_started_ms = millis;
            if state.phase == LoadingPhase::Downloading {
                self.fraction = 0.0;
            }
            self.phase = Some(state.phase);
        }
        let elapsed = millis.saturating_sub(self.phase_started_ms) as f64;
        let activity = elapsed / (elapsed + 3000.0);
        let target = match state.phase {
            LoadingPhase::Downloading => state
                .download
                .fraction()
                .map(|fraction| DOWNLOAD_SHARE * fraction)
                .unwrap_or((DOWNLOAD_SHARE - ACTIVITY_HEADROOM) * activity),
            LoadingPhase::Preparing => {
                DOWNLOAD_SHARE + (1.0 - ACTIVITY_HEADROOM - DOWNLOAD_SHARE) * activity
            }
            // Freeze the base width so @out can fill the remainder smoothly.
            LoadingPhase::Ready | LoadingPhase::Revealing => self.fraction.max(DOWNLOAD_SHARE),
            LoadingPhase::Active => 1.0,
            LoadingPhase::Failed | LoadingPhase::Cancelled => self.fraction,
        };
        self.fraction = self.fraction.max(target);
        self.fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_and_preparation_share_a_monotonic_fill_with_room_for_exit() {
        let mut bar = LoadingBarProgress::default();
        let mut state = LoadingState::default();
        state.download.total_bytes = Some(100);
        state.download.received_bytes = 50;
        assert_eq!(bar.advance(&state, 0), 0.335);
        state.download.received_bytes = 100;
        assert_eq!(bar.advance(&state, 100), 0.67);
        state.phase = LoadingPhase::Preparing;
        assert_eq!(bar.advance(&state, 100), 0.67);
        let mut previous = 0.67;
        for millis in [200, 1100, 3100, 10100, 600100] {
            let next = bar.advance(&state, millis);
            assert!(next > previous && next < 0.95);
            previous = next;
        }
        for phase in [LoadingPhase::Ready, LoadingPhase::Revealing] {
            state.phase = phase;
            assert_eq!(bar.advance(&state, 700000), previous);
        }
    }

    #[test]
    fn unknown_download_never_recedes_when_a_total_arrives_and_retry_resets() {
        let mut bar = LoadingBarProgress::default();
        let mut state = LoadingState::default();
        assert_eq!(bar.advance(&state, 0), 0.0);
        let activity = bar.advance(&state, 3000);
        assert!(activity > 0.0 && activity < 0.67);
        state.download.total_bytes = Some(100);
        state.download.received_bytes = 1;
        assert_eq!(bar.advance(&state, 3001), activity);
        state.phase = LoadingPhase::Failed;
        assert_eq!(bar.advance(&state, 9000), activity);
        state.phase = LoadingPhase::Downloading;
        state.download.received_bytes = 0;
        assert_eq!(bar.advance(&state, 10000), 0.0);
    }
}
