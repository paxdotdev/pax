pub use crate::{Error, InitOptions};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use crate::library::{open, symbol};

struct NativeApi {
    _handle: usize,
    configure: unsafe extern "C" fn(*const u8, usize) -> i32,
    pax_host_version: unsafe extern "C" fn() -> u32,
    pax_host_prepare_with_data: unsafe extern "C" fn(*const u8, usize) -> *mut c_void,
    pax_host_request: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        usize,
        *mut c_void,
        extern "C" fn(*mut c_void, u32, *const u8, usize),
    ),
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    pax_host_window_options:
        unsafe extern "C" fn(*mut c_void, *const u8, usize, f64, f64, f64, f64, bool) -> i32,
    pax_host_run: unsafe extern "C" fn(*mut c_void) -> i32,
    pax_host_mount: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
    pax_host_observe: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        extern "C" fn(*mut c_void, u32),
        release: extern "C" fn(*mut c_void),
    ),
    pax_host_close: unsafe extern "C" fn(*mut c_void),
    pax_host_release: unsafe extern "C" fn(*mut c_void),
    pax_host_error: unsafe extern "C" fn(*mut u8, usize) -> usize,
}
static NATIVE_API: std::sync::OnceLock<Result<NativeApi, String>> = std::sync::OnceLock::new();
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn bundled_framework(name: &str) -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let frameworks = if directory.file_name()? == "MacOS" {
        directory.parent()?.join("Frameworks")
    } else {
        directory.join("Frameworks")
    };
    let path = frameworks.join(format!("{name}.framework/{name}"));
    path.is_file().then_some(path)
}

fn host_library_path() -> Result<std::path::PathBuf, String> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let name = if cfg!(target_os = "windows") {
            "PaxHost.dll"
        } else {
            "libpax_host.so"
        };
        if let Some(path) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
            .filter(|path| path.is_file())
        {
            return Ok(path);
        }
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    if let Some(path) = bundled_framework("PaxHost") {
        return Ok(path);
    }
    if let Some(path) = option_env!("PAX_HOST_SDK").map(std::path::PathBuf::from) {
        return Ok(path);
    }
    Err("PaxHost is missing. Enable pax-kit/host for native Cargo builds, or package the native application.".into())
}

fn configure(value: &serde_json::Value) -> Result<(), Error> {
    let api = native_api()?;
    let bytes = serde_json::to_vec(value).map_err(|e| Error(e.to_string()))?;
    if unsafe { (api.configure)(bytes.as_ptr(), bytes.len()) } != 0 {
        return Err(last_error());
    }
    Ok(())
}

// Names are supplied by the compiled UI's metadata, never inferred from a
// dependency alias, checkout path, or the contents of the loaded library.
#[cfg(any(
    test,
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios"
))]
fn native_export(recipe: &serde_json::Value, field: &str) -> Result<std::ffi::CString, Error> {
    let name = recipe["nativeExports"][field].as_str()
        .filter(|name| !name.is_empty() && name.len() <= 128
            && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
        .ok_or_else(|| Error(format!("Missing or invalid native export `{field}`; rebuild the launcher, UI library and native host together")))?;
    std::ffi::CString::new(name).map_err(|error| Error(error.to_string()))
}

/// Used by the facade's launch macros to carry copied build metadata across
/// the native ABI. The application library is never called through Rust.
#[doc(hidden)]
pub fn __init_cartridge(recipe: &str, options: InitOptions) -> Result<PreparedApp, Error> {
    __init_cartridge_with_fonts(recipe, options, &[])
}

/// Generated launcher entry; embedded fonts are staged only for direct Windows
/// Cargo launches. Packaged applications use their existing resource directory.
#[doc(hidden)]
pub fn __init_cartridge_with_fonts(
    recipe: &str,
    options: InitOptions,
    fonts: &[(usize, &str, &str, &[u8])],
) -> Result<PreparedApp, Error> {
    #[cfg(not(target_os = "windows"))]
    let _ = fonts;
    let mut recipe: serde_json::Value =
        serde_json::from_str(recipe).map_err(|e| Error(e.to_string()))?;
    recipe["entry"] = options.entry.as_deref().unwrap_or("").into();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    configure_desktop_recipe(&mut recipe)?;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    if bundled_framework("PaxCartridge").is_none() {
        let executable = std::env::current_exe().map_err(|e| Error(e.to_string()))?;
        let directory = executable
            .parent()
            .ok_or_else(|| Error("Executable has no directory".into()))?;
        let profile = if directory.file_name().is_some_and(|name| name == "examples") {
            directory.parent().unwrap()
        } else {
            directory
        };
        let library = recipe["library"]
            .as_str()
            .ok_or_else(|| Error("Missing UI library name".into()))?;
        let cartridge = profile
            .join("deps")
            .join(format!("lib{}.dylib", library.replace('-', "_")));
        if !cartridge.is_file() {
            return Err(Error(format!("Missing UI library {}. Build the launcher and its cdylib together with Cargo, or package them with pax-cli.", cartridge.display())));
        }
        recipe["cartridge"] = cartridge.to_string_lossy().as_ref().into();
    } else {
        // Packaged resources and libraries are relocatable; source-tree paths
        // in development metadata must not override a signed bundle.
        recipe["assets"] = serde_json::json!([]);
        recipe["programId"] = serde_json::Value::Null;
    }
    #[cfg(target_os = "windows")]
    {
        let executable = std::env::current_exe().map_err(|error| Error(error.to_string()))?;
        crate::fonts::stage(
            &mut recipe,
            executable
                .parent()
                .ok_or_else(|| Error("Executable has no directory".into()))?,
            fonts,
        )?;
    }
    for field in [
        "abiVersion",
        "programId",
        "init",
        "initWithData",
        "initEntry",
    ] {
        native_export(&recipe, field)?;
    }
    configure(&recipe)?;
    prepare(options)
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn configure_desktop_recipe(recipe: &mut serde_json::Value) -> Result<(), Error> {
    let executable = std::env::current_exe().map_err(|error| Error(error.to_string()))?;
    let directory = executable
        .parent()
        .ok_or_else(|| Error("Executable has no directory".into()))?;
    resolve_desktop_recipe(recipe, directory, cfg!(target_os = "windows"))?;
    if let Some(expected) = recipe["programId"].as_str() {
        let cartridge = recipe["cartridge"]
            .as_str()
            .ok_or_else(|| Error("Missing desktop cartridge path".into()))?;
        // Cargo may rebuild only the launcher. Check its independently loaded
        // library before initialization so an old cdylib cannot silently run.
        unsafe {
            let handle = open(std::path::Path::new(cartridge)).map_err(Error)?;
            let abi: unsafe extern "C" fn() -> u32 = symbol(
                handle,
                native_export(recipe, "abiVersion")?.as_bytes_with_nul(),
            )
            .map_err(Error)?;
            if abi() != 4 {
                return Err(Error(
                    "Desktop cartridge requires ABI 4; rebuild its cdylib and launcher together"
                        .into(),
                ));
            }
            let identity: unsafe extern "C" fn(*mut u8, usize) -> usize = symbol(
                handle,
                native_export(recipe, "programId")?.as_bytes_with_nul(),
            )
            .map_err(Error)?;
            validate_program_identity(identity, expected)?;
        }
    }
    Ok(())
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn validate_program_identity(
    identity: unsafe extern "C" fn(*mut u8, usize) -> usize,
    expected: &str,
) -> Result<(), Error> {
    let len = unsafe { identity(std::ptr::null_mut(), 0) };
    if len == 0 || len > 4096 {
        return Err(Error("Invalid desktop cartridge program identity".into()));
    }
    let mut bytes = vec![0; len];
    if unsafe { identity(bytes.as_mut_ptr(), bytes.len()) } != len || bytes != expected.as_bytes() {
        return Err(Error("Desktop cartridge does not match this launcher. Build its cdylib and launcher together with Cargo, or package them with pax-cli.".into()));
    }
    Ok(())
}

#[cfg(any(test, target_os = "windows", target_os = "linux"))]
fn resolve_desktop_recipe(
    recipe: &mut serde_json::Value,
    directory: &std::path::Path,
    windows: bool,
) -> Result<(), Error> {
    let package = directory.join("pax-app.json");
    if package.is_file() {
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&package).map_err(|error| Error(error.to_string()))?,
        )
        .map_err(|error| Error(error.to_string()))?;
        let resolve = |value: &serde_json::Value| -> Result<String, Error> {
            let relative = std::path::Path::new(
                value
                    .as_str()
                    .ok_or_else(|| Error("Invalid Pax package resource path".into()))?,
            );
            if relative.as_os_str().is_empty()
                || relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(Error("Pax package resource paths must be relative".into()));
            }
            Ok(directory.join(relative).to_string_lossy().into_owned())
        };
        recipe["cartridge"] = resolve(&metadata["cartridge"])?.into();
        recipe["assets"] = serde_json::Value::Array(
            metadata["assets"]
                .as_array()
                .ok_or_else(|| Error("Missing Pax package asset roots".into()))?
                .iter()
                .map(|asset| resolve(asset).map(Into::into))
                .collect::<Result<_, _>>()?,
        );
        recipe["programId"] = serde_json::Value::Null;
        recipe["nativeExports"] = metadata["nativeExports"].clone();
    } else {
        let library = recipe["library"]
            .as_str()
            .ok_or_else(|| Error("Missing UI library name".into()))?
            .replace('-', "_");
        let name = if windows {
            format!("{library}.dll")
        } else {
            format!("lib{library}.so")
        };
        let profile = if directory.file_name().is_some_and(|name| name == "examples") {
            directory.parent().unwrap()
        } else {
            directory
        };
        let cartridge = [profile.join(&name), profile.join("deps").join(&name)].into_iter().find(|path| path.is_file()).ok_or_else(|| Error(format!("Missing UI library {name}. Build the launcher and its cdylib together with Cargo, or package them with pax-cli.")))?;
        recipe["cartridge"] = cartridge.to_string_lossy().as_ref().into();
    }
    Ok(())
}
fn native_api() -> Result<&'static NativeApi, Error> {
    NATIVE_API
        .get_or_init(|| unsafe {
            let handle = open(&host_library_path()?)?;
            Ok(NativeApi {
                _handle: handle as usize,
                configure: symbol(handle, b"pax_host_configure\0")?,
                pax_host_version: symbol(handle, b"pax_host_version\0")?,
                pax_host_prepare_with_data: symbol(handle, b"pax_host_prepare_with_data\0")?,
                pax_host_request: symbol(handle, b"pax_host_request\0")?,
                #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
                pax_host_window_options: symbol(handle, b"pax_host_window_options\0")?,
                pax_host_run: symbol(handle, b"pax_host_run\0")?,
                pax_host_mount: symbol(handle, b"pax_host_mount\0")?,
                pax_host_observe: symbol(handle, b"pax_host_observe\0")?,
                pax_host_close: symbol(handle, b"pax_host_close\0")?,
                pax_host_release: symbol(handle, b"pax_host_release\0")?,
                pax_host_error: symbol(handle, b"pax_host_error\0")?,
            })
        })
        .as_ref()
        .map_err(|error| Error(error.clone()))
}
unsafe fn pax_host_version() -> u32 {
    (native_api().expect("host was prepared").pax_host_version)()
}
unsafe fn pax_host_prepare_with_data(data: *const u8, len: usize) -> *mut c_void {
    (native_api()
        .expect("host was prepared")
        .pax_host_prepare_with_data)(data, len)
}
unsafe fn pax_host_request(
    session: *mut c_void,
    data: *const u8,
    len: usize,
    context: *mut c_void,
    reply: extern "C" fn(*mut c_void, u32, *const u8, usize),
) {
    (native_api().expect("host was prepared").pax_host_request)(session, data, len, context, reply)
}
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
unsafe fn pax_host_window_options(
    session: *mut c_void,
    title: *const u8,
    len: usize,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    resizable: bool,
) -> i32 {
    (native_api()
        .expect("host was prepared")
        .pax_host_window_options)(
        session, title, len, width, height, min_width, min_height, resizable,
    )
}
unsafe fn pax_host_run(session: *mut c_void) -> i32 {
    (native_api().expect("host was prepared").pax_host_run)(session)
}
unsafe fn pax_host_mount(session: *mut c_void, view: *mut c_void, controller: *mut c_void) -> i32 {
    (native_api().expect("host was prepared").pax_host_mount)(session, view, controller)
}
unsafe fn pax_host_observe(
    session: *mut c_void,
    context: *mut c_void,
    notify: extern "C" fn(*mut c_void, u32),
    release: extern "C" fn(*mut c_void),
) {
    (native_api().expect("host was prepared").pax_host_observe)(session, context, notify, release)
}
unsafe fn pax_host_close(session: *mut c_void) {
    (native_api().expect("host was prepared").pax_host_close)(session)
}
unsafe fn pax_host_release(session: *mut c_void) {
    (native_api().expect("host was prepared").pax_host_release)(session)
}
unsafe fn pax_host_error(buffer: *mut u8, capacity: usize) -> usize {
    (native_api().expect("host was prepared").pax_host_error)(buffer, capacity)
}

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
    // release. Never unwind into the native host, including through an executor's waker.
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

/// Prepare a cartridge with explicit encoded inputs. No Rust allocation or
/// object identity is shared with the cartridge. See [`init`] for lifecycle.
pub fn init_with(options: InitOptions) -> Result<PreparedApp, Error> {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        return __init_cartridge("{}", options);
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        configure(&serde_json::json!({"entry": options.entry.as_deref().unwrap_or("")}))?;
        prepare(options)
    }
}

fn prepare(options: InitOptions) -> Result<PreparedApp, Error> {
    native_api()?;
    unsafe {
        if pax_host_version() != 2 {
            return Err(Error("incompatible PaxHost native library".into()));
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
/// The public embedding contract covers Apple targets. The lower-level GTK
/// mounting path is experimental and has not been qualified for application use.
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
/// Standalone desktop window policy, expressed in logical points. iOS/iPadOS
/// windows are controlled by their scene; embedded mounts use their host bounds.
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
pub struct WindowOptions {
    /// None preserves the application title from its project metadata.
    pub title: Option<String>,
    /// Initial content width and height in logical pixels.
    pub size: (f64, f64),
    /// Minimum content width and height in logical pixels.
    pub minimum_size: (f64, f64),
    /// Whether the user may resize the standalone window.
    pub resizable: bool,
}
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
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
    /// Enter the standalone desktop loop with an explicit window policy.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
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

    /// Enter the platform's standalone application loop on the main
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

#[cfg(test)]
mod package_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_exports_require_explicit_valid_metadata() {
        let mut recipe = json!({"nativeExports": {"initEntry": "pax_init_entry_selected"}});
        assert_eq!(
            native_export(&recipe, "initEntry").unwrap().to_bytes(),
            b"pax_init_entry_selected"
        );
        assert!(native_export(&recipe, "abiVersion").is_err());
        assert!(native_export(&json!({}), "initEntry").is_err());
        for name in [
            json!(null),
            json!(12),
            json!(""),
            json!("pax_init\u{0}other"),
            json!("pax::init"),
            json!("é"),
            json!("x".repeat(129)),
        ] {
            recipe["nativeExports"]["initEntry"] = name;
            assert!(native_export(&recipe, "initEntry").is_err());
        }
    }

    #[test]
    fn direct_cargo_rejects_stale_or_oversized_program_identities() {
        unsafe extern "C" fn identity(data: *mut u8, capacity: usize) -> usize {
            let value = b"current-program";
            if !data.is_null() && capacity >= value.len() {
                unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), data, value.len()) };
            }
            value.len()
        }
        unsafe extern "C" fn oversized(_: *mut u8, _: usize) -> usize {
            usize::MAX
        }
        assert!(validate_program_identity(identity, "current-program").is_ok());
        assert!(validate_program_identity(identity, "previous-program")
            .unwrap_err()
            .to_string()
            .contains("does not match this launcher"));
        assert!(validate_program_identity(oversized, "current-program").is_err());
    }

    #[test]
    fn relocated_packages_replace_source_paths_and_preserve_asset_precedence() {
        let package = tempfile::tempdir().unwrap();
        std::fs::write(
            package.path().join("pax-app.json"),
            serde_json::to_vec(&json!({
                "cartridge": "app.dll", "assets": ["assets/0", "assets/1"],
                "nativeExports": {"initEntry": "pax_init_entry_packaged"}
            }))
            .unwrap(),
        )
        .unwrap();
        let mut recipe = json!({"library":"app","programId":"source-build", "assets":["/old/source/assets"],"entry":"crate::Root", "nativeExports":{"initEntry":"pax_init_entry_source"}});
        resolve_desktop_recipe(&mut recipe, package.path(), true).unwrap();
        assert_eq!(
            recipe["cartridge"],
            package.path().join("app.dll").to_string_lossy().as_ref()
        );
        assert_eq!(
            recipe["assets"],
            json!([
                package.path().join("assets/0"),
                package.path().join("assets/1")
            ])
        );
        assert!(recipe["programId"].is_null());
        assert_eq!(recipe["entry"], "crate::Root");
        assert_eq!(
            native_export(&recipe, "initEntry").unwrap().to_bytes(),
            b"pax_init_entry_packaged"
        );
    }

    #[test]
    fn package_paths_cannot_escape_the_package_directory() {
        let package = tempfile::tempdir().unwrap();
        for path in ["../app.dll", "/tmp/app.dll"] {
            std::fs::write(
                package.path().join("pax-app.json"),
                serde_json::to_vec(&json!({"cartridge":path,"assets":[]})).unwrap(),
            )
            .unwrap();
            assert!(resolve_desktop_recipe(&mut json!({}), package.path(), true).is_err());
        }
    }

    #[test]
    fn cargo_examples_find_platform_cdylibs_without_erasing_source_metadata() {
        for (windows, filename) in [(true, "chosen_ui.dll"), (false, "libchosen_ui.so")] {
            let profile = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(profile.path().join("deps")).unwrap();
            std::fs::create_dir_all(profile.path().join("examples")).unwrap();
            std::fs::write(profile.path().join("deps").join(filename), b"fixture").unwrap();
            let mut recipe = json!({"library":"chosen-ui","programId":"source-build","assets":["/source/assets"],"nativeExports":{"initEntry":"pax_init_entry_source"}});
            resolve_desktop_recipe(&mut recipe, &profile.path().join("examples"), windows).unwrap();
            assert_eq!(
                recipe["cartridge"],
                profile
                    .path()
                    .join("deps")
                    .join(filename)
                    .to_string_lossy()
                    .as_ref()
            );
            assert_eq!(recipe["programId"], "source-build");
            assert_eq!(recipe["assets"], json!(["/source/assets"]));
            assert_eq!(
                native_export(&recipe, "initEntry").unwrap().to_bytes(),
                b"pax_init_entry_source"
            );
        }
    }
}
