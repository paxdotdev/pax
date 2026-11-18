//! Plain-text clipboard writes with chassis-owned access and scoped completion.
use crate::{AsyncError, AsyncScope};
use std::rc::Rc;

/// Why a plain-text clipboard write did not complete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    /// The chassis has no clipboard adapter, or the platform API is unavailable.
    Unsupported,
    /// The platform rejected access (for example, permission or user activation).
    NotAllowed,
    /// The platform reported another write failure.
    Failed,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "clipboard writing is unavailable on this target",
            Self::NotAllowed => "the platform did not allow clipboard access",
            Self::Failed => "the clipboard write failed",
        })
    }
}
impl std::error::Error for ClipboardError {}

/// A chassis must call this exactly once after writing or failing. It may call
/// synchronously; user callbacks are always deferred through the UI scope.
pub type ClipboardReply = Box<dyn FnOnce(Result<(), ClipboardError>)>;

/// Per-engine clipboard adapter. This is not a process-global clipboard cache.
///
/// Chassis adapters start the platform request synchronously to preserve browser
/// user activation, then report its actual completion. No clipboard reads occur.
#[derive(Clone, Default)]
pub struct Clipboard {
    writer: Option<Rc<dyn Fn(String, ClipboardReply)>>,
}

impl Clipboard {
    /// Construct a chassis adapter; an unconfigured clipboard reports Unsupported.
    pub fn new(writer: impl Fn(String, ClipboardReply) + 'static) -> Self {
        Self {
            writer: Some(Rc::new(writer)),
        }
    }

    /// Start a write now and deliver its result on the UI thread while `scope`
    /// remains alive. `Ok(())` means accepted, not copied. Closing the scope
    /// suppresses the callback, but cannot undo a platform write already started.
    pub fn write_text(
        &self,
        scope: &AsyncScope,
        text: String,
        callback: impl FnOnce(Result<(), ClipboardError>) + 'static,
    ) -> Result<(), AsyncError> {
        let completion = scope.completion(callback)?;
        let reply: ClipboardReply = Box::new(move |result| {
            let _ = completion.complete(result);
        });
        match &self.writer {
            Some(write) => write(text, reply),
            None => reply(Err(ClipboardError::Unsupported)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_runtime::AsyncDispatcher;
    use crate::AsyncLimits;
    use std::cell::RefCell;

    #[test]
    fn starts_inline_but_delivers_confirmation_later() {
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let written = Rc::new(RefCell::new(String::new()));
        let output = written.clone();
        let clipboard = Clipboard::new(move |text, reply| {
            *output.borrow_mut() = text;
            reply(Ok(()));
        });
        let result = Rc::new(RefCell::new(None));
        let delivered = result.clone();
        clipboard
            .write_text(&scope, "café ☕\n100%".into(), move |r| {
                *delivered.borrow_mut() = Some(r);
            })
            .unwrap();
        assert_eq!(&*written.borrow(), "café ☕\n100%");
        assert!(result.borrow().is_none());
        dispatcher.drain(|| {}).unwrap();
        assert_eq!(*result.borrow(), Some(Ok(())));
    }

    #[test]
    fn unsupported_and_denied_are_not_successes() {
        for (clipboard, expected) in [
            (Clipboard::default(), ClipboardError::Unsupported),
            (
                Clipboard::new(|_, reply| reply(Err(ClipboardError::NotAllowed))),
                ClipboardError::NotAllowed,
            ),
        ] {
            let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
            let scope = dispatcher.scope().unwrap();
            dispatcher.activate();
            let result = Rc::new(RefCell::new(None));
            let delivered = result.clone();
            clipboard
                .write_text(&scope, String::new(), move |r| {
                    *delivered.borrow_mut() = Some(r);
                })
                .unwrap();
            dispatcher.drain(|| {}).unwrap();
            assert_eq!(*result.borrow(), Some(Err(expected)));
        }
    }

    #[test]
    fn late_completion_is_discarded_after_unmount() {
        let dispatcher = AsyncDispatcher::new(AsyncLimits::default());
        let scope = dispatcher.scope().unwrap();
        dispatcher.activate();
        let pending = Rc::new(RefCell::new(None));
        let saved = pending.clone();
        let clipboard = Clipboard::new(move |_, reply| *saved.borrow_mut() = Some(reply));
        clipboard
            .write_text(&scope, "text".into(), |_| panic!("detached callback"))
            .unwrap();
        scope.cancel();
        pending.borrow_mut().take().unwrap()(Ok(()));
        dispatcher.drain(|| {}).unwrap();
        assert!(clipboard
            .write_text(&scope, "again".into(), |_| {})
            .is_err());
        assert!(pending.borrow().is_none());
    }
}
