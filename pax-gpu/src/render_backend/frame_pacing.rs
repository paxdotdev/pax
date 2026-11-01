//! Bound unfinished scene frames without blocking the host or queuing scene snapshots.
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

const MAX_PENDING_FRAMES: usize = 2;
static NEXT_FRAME: AtomicU64 = AtomicU64::new(1);

/// Identity shared by every rendering slice of one settled application frame.
/// Create a new identity after each tick, and reuse it across layers and continuations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderFrame(u64);
impl Default for RenderFrame {
    fn default() -> Self {
        Self(NEXT_FRAME.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Default)]
struct State {
    pending: HashMap<RenderFrame, u64>,
    serial: u64,
}
#[derive(Default)]
pub(super) struct FramePacing {
    state: Arc<Mutex<State>>,
}
impl FramePacing {
    pub(super) fn ready(&self, frame: RenderFrame) -> bool {
        let state = self.state.lock().unwrap();
        state.pending.contains_key(&frame) || state.pending.len() < MAX_PENDING_FRAMES
    }
    pub(super) fn submitted(&self, frame: RenderFrame) -> Completion {
        let mut state = self.state.lock().unwrap();
        state.serial += 1;
        let serial = state.serial;
        state.pending.insert(frame, serial);
        Completion {
            state: Arc::clone(&self.state),
            frame,
            serial,
        }
    }
}

// A later cooperative slice extends the same frame's fence. An earlier callback
// must not release its allowance while the newer slice is still executing.
// Cancellation also releases the latest fence, without retaining the GPU device.
pub(super) struct Completion {
    state: Arc<Mutex<State>>,
    frame: RenderFrame,
    serial: u64,
}
impl Drop for Completion {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        if state.pending.get(&self.frame) == Some(&self.serial) {
            state.pending.remove(&self.frame);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_reopens_capacity_without_queueing_more_frames() {
        let pacing = FramePacing::default();
        let a = RenderFrame::default();
        let b = RenderFrame::default();
        let latest = RenderFrame::default();
        let first = pacing.submitted(a);
        let second = pacing.submitted(b);
        for _ in 0..100 {
            assert!(!pacing.ready(latest));
        }
        assert!(
            pacing.ready(b),
            "all layers of an admitted frame can finish"
        );
        let continuation = pacing.submitted(b);
        drop(second);
        assert!(
            !pacing.ready(latest),
            "old completion cannot retire a newer slice"
        );
        drop(first);
        assert!(pacing.ready(latest));
        let newest = pacing.submitted(latest);
        assert!(!pacing.ready(RenderFrame::default()));
        drop(continuation);
        drop(newest);
        assert!(pacing.ready(RenderFrame::default()));
    }
}
