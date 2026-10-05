use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    fmt,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    rc::Rc,
    task::{Context, Poll, Waker},
};

// Unit tests of platform-independent policy do not need a packaged framework.
#[cfg_attr(not(test), link(name = "PaxHost", kind = "framework"))]
extern "C" {
    fn pax_host_version() -> u32;
    fn pax_host_prepare_with_data(data: *const u8, len: usize) -> *mut c_void;
    fn pax_host_request(
        session: *mut c_void,
        data: *const u8,
        len: usize,
        context: *mut c_void,
        reply: extern "C" fn(*mut c_void, u32, *const u8, usize),
    );
    #[cfg(target_os = "macos")]
    fn pax_host_window_options(
        session: *mut c_void,
        title: *const u8,
        len: usize,
        width: f64,
        height: f64,
        min_width: f64,
        min_height: f64,
        resizable: bool,
    ) -> i32;
    fn pax_host_run(session: *mut c_void) -> i32;
    fn pax_host_mount(session: *mut c_void, view: *mut c_void, controller: *mut c_void) -> i32;
    fn pax_host_observe(
        session: *mut c_void,
        context: *mut c_void,
        notify: extern "C" fn(*mut c_void, u32),
        release: extern "C" fn(*mut c_void),
    );
    fn pax_host_close(session: *mut c_void);
    fn pax_host_release(session: *mut c_void);
    fn pax_host_error(buffer: *mut u8, capacity: usize) -> usize;
}

/// An initialization or mounting failure reported by the native host.
#[derive(Debug, Clone)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
fn last_error() -> Error {
    unsafe {
        let size = pax_host_error(std::ptr::null_mut(), 0);
        let mut bytes = vec![0; size];
        pax_host_error(bytes.as_mut_ptr(), size);
        Error(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[derive(Default)]
struct Observation {
    status: Cell<u32>,
    waiters: RefCell<Vec<std::rc::Weak<RefCell<Option<Waker>>>>>,
}
extern "C" fn notify(context: *mut c_void, status: u32) {
    // The host invokes this only on the main thread and retains the Rc until
    // release. Never unwind into Swift, including through an executor's waker.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { &*(context as *const Observation) };
        state.status.set(status);
        let waiters = state
            .waiters
            .borrow()
            .iter()
            .filter_map(|w| w.upgrade())
            .collect::<Vec<_>>();
        for waiter in waiters {
            let wake = waiter.borrow_mut().take();
            if let Some(wake) = wake {
                wake.wake();
            }
        }
    }));
    if result.is_err() {
        std::process::abort();
    }
}
extern "C" fn release(context: *mut c_void) {
    unsafe {
        drop(Rc::from_raw(context as *const Observation));
    }
}

/// A prepared cartridge, confined to the platform's main thread. Consume it
/// once with `run` or `mount`. Dropping it starts retained asynchronous cleanup.
pub struct PreparedApp {
    session: NonNull<c_void>,
    state: Rc<Observation>,
}

/// Prepare the bundled cartridge and invoke its typed root initializer once.
/// Must be called on the process main thread. No window or nested event loop is
/// created. Initializers should return a Loading root and stage asynchronous
/// work; staged work becomes eligible when the host is ready.
pub fn init() -> Result<PreparedApp, Error> {
    init_with(InitOptions::default())
}

/// Copied application inputs for the separately linked cartridge.
#[derive(Default)]
pub struct InitOptions {
    /// Application-defined encoding, at most 64 KiB. The root initializer
    /// decodes these bytes through `InitContext::startup_data()`.
    pub startup_data: Vec<u8>,
}

/// Prepare a cartridge with explicit encoded inputs. No Rust allocation or
/// object identity is shared with the cartridge. See [`init`] for lifecycle.
pub fn init_with(options: InitOptions) -> Result<PreparedApp, Error> {
    unsafe {
        if pax_host_version() != 1 {
            return Err(Error("incompatible PaxHost framework".into()));
        }
        let session = NonNull::new(pax_host_prepare_with_data(
            options.startup_data.as_ptr(),
            options.startup_data.len(),
        ))
        .ok_or_else(last_error)?;
        let state = Rc::new(Observation::default());
        pax_host_observe(
            session.as_ptr(),
            Rc::into_raw(state.clone()) as *mut c_void,
            notify,
            release,
        );
        Ok(PreparedApp { session, state })
    }
}

/// A caller-owned native view and (on iOS) its containing view controller.
/// This borrow prevents the Rust wrapper from outliving the supplied owner.
pub struct ViewHost<'a> {
    view: NonNull<c_void>,
    controller: *mut c_void,
    _owner: PhantomData<&'a mut Rc<()>>,
}
impl<'a> ViewHost<'a> {
    /// Wrap a native container. `owner` must own the NSView on macOS, or both
    /// UIView and UIViewController on iOS, for the entire mount lifetime.
    ///
    /// # Safety
    /// Pointers must have those Objective-C types, belong to the process main
    /// thread, and remain valid until the mount is dropped. On iOS the controller
    /// must own the container in its view hierarchy. Do not reparent a live mount.
    pub unsafe fn from_raw<T>(
        _: &'a mut T,
        view: NonNull<c_void>,
        controller: *mut c_void,
    ) -> Self {
        Self {
            view,
            controller,
            _owner: PhantomData,
        }
    }
}
/// Standalone macOS window policy, expressed in logical points. iOS/iPadOS
/// windows are controlled by their scene; embedded mounts use their host bounds.
#[cfg(target_os = "macos")]
pub struct WindowOptions {
    /// None preserves the application's bundle display name.
    pub title: Option<String>,
    pub size: (f64, f64),
    pub minimum_size: (f64, f64),
    pub resizable: bool,
}
#[cfg(target_os = "macos")]
impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: None,
            size: (900.0, 450.0),
            minimum_size: (0.0, 0.0),
            resizable: true,
        }
    }
}
impl PreparedApp {
    /// Enter the standalone macOS loop with an explicit window policy.
    #[cfg(target_os = "macos")]
    pub fn run_with(self, options: WindowOptions) -> Result<(), Error> {
        let title = options.title.as_deref().unwrap_or("").as_bytes();
        let result = unsafe {
            pax_host_window_options(
                self.session.as_ptr(),
                title.as_ptr(),
                title.len(),
                options.size.0,
                options.size.1,
                options.minimum_size.0,
                options.minimum_size.1,
                options.resizable,
            )
        };
        if result != 0 {
            return Err(last_error());
        }
        self.run()
    }

    /// Enter the platform's standalone SwiftUI application loop on the main
    /// thread. On iOS this normally does not return. Configure process services
    /// before calling this; cartridge-local services belong in its initializer.
    pub fn run(self) -> Result<(), Error> {
        let result = unsafe { pax_host_run(self.session.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(last_error())
        }
    }
    /// Attach the full Pax hierarchy, including native controls, to a region of
    /// the caller's view. The caller continues to own its window and event loop.
    pub fn mount<'a>(self, host: ViewHost<'a>) -> Result<MountedApp<'a>, Error> {
        let result =
            unsafe { pax_host_mount(self.session.as_ptr(), host.view.as_ptr(), host.controller) };
        if result != 0 {
            return Err(last_error());
        }
        Ok(MountedApp {
            app: self,
            _owner: PhantomData,
        })
    }
}
impl Drop for PreparedApp {
    fn drop(&mut self) {
        unsafe {
            pax_host_close(self.session.as_ptr());
            pax_host_release(self.session.as_ptr());
        }
    }
}

/// An owned single-region mount. Drop or `close` removes only Pax's child
/// hierarchy and revokes its work. Cleanup continues through host wakeups even
/// if every observer is dropped or the region is hidden.
pub struct MountedApp<'a> {
    app: PreparedApp,
    _owner: PhantomData<&'a mut Rc<()>>,
}
impl MountedApp<'_> {
    /// Send an application-defined request and observe its reply without
    /// blocking the UI thread. Each buffer is limited to 64 KiB and copied
    /// across the boundary. Close rejects queued requests; dropping this future
    /// discards observation without cancelling already accepted work.
    pub fn request(&self, bytes: &[u8]) -> Request {
        let state = Rc::new(RequestState::default());
        unsafe {
            pax_host_request(
                self.app.session.as_ptr(),
                bytes.as_ptr(),
                bytes.len(),
                Rc::into_raw(state.clone()) as *mut c_void,
                request_reply,
            );
        }
        Request(state)
    }

    /// Observe host readiness, including the property wake route and viewport.
    pub fn ready(&self) -> Completion {
        Completion::new(self.app.state.clone(), false)
    }
    /// Start close without blocking the host thread.
    pub fn close(&self) {
        unsafe {
            pax_host_close(self.app.session.as_ptr());
        }
    }
    /// Observe cleanup acknowledgement. Polling does not drive shutdown.
    pub fn closed(&self) -> Completion {
        Completion::new(self.app.state.clone(), true)
    }
}

/// A readiness/cleanup observer. It may be dropped without cancelling cleanup.
pub struct Completion {
    state: Rc<Observation>,
    waiter: Rc<RefCell<Option<Waker>>>,
    closing: bool,
}
impl Completion {
    fn new(state: Rc<Observation>, closing: bool) -> Self {
        let waiter = Rc::new(RefCell::new(None));
        state.waiters.borrow_mut().retain(|w| w.strong_count() != 0);
        state.waiters.borrow_mut().push(Rc::downgrade(&waiter));
        Self {
            state,
            waiter,
            closing,
        }
    }
}
impl Future for Completion {
    type Output = Result<(), Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match (self.closing, self.state.status.get()) {
            (true, 4) | (false, 2) => Poll::Ready(Ok(())),
            (false, 3..=5) => Poll::Ready(Err(Error("mount closed before readiness".into()))),
            _ => {
                *self.waiter.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[derive(Default)]
struct RequestState {
    result: RefCell<Option<Result<Vec<u8>, Error>>>,
    wake: RefCell<Option<Waker>>,
}
extern "C" fn request_reply(context: *mut c_void, status: u32, data: *const u8, len: usize) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let state = unsafe { Rc::from_raw(context as *const RequestState) };
        let bytes = if len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
        };
        *state.result.borrow_mut() = Some(if status == 0 {
            Ok(bytes)
        } else {
            Err(Error(String::from_utf8_lossy(&bytes).into_owned()))
        });
        let wake = state.wake.borrow_mut().take();
        if let Some(wake) = wake {
            wake.wake();
        }
    }));
    if result.is_err() {
        std::process::abort();
    }
}

/// Observation of one host request. Poll on the host's UI executor.
pub struct Request(Rc<RequestState>);
impl Future for Request {
    type Output = Result<Vec<u8>, Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(result) = self.0.result.borrow_mut().take() {
            return Poll::Ready(result);
        }
        *self.0.wake.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}
