//! Application launch and native embedding through the Pax facade.
//!
//! `init!()?.run()` starts a development session when `designtime` is enabled
//! on a workstation. Packaged applications execute their native entry instead.
//! Use `init_with!` for explicit in-process initialization and embedding.

pub use pax_host::*;

/// A prepared native application or a workstation development session.
///
/// Preparing a development session does not initialize the target UI. Its root
/// initializer runs in the browser or native application after launch.
pub struct Application(Entry);

enum Entry {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    Native(pax_host::PreparedApp),
    #[cfg(all(
        feature = "designtime",
        any(target_os = "macos", target_os = "linux", target_os = "windows")
    ))]
    Development(pax_compiler::launcher::Launch),
}

impl Application {
    /// Run the selected development session or enter the native application loop.
    pub fn run(self) -> Result<(), Error> {
        match self.0 {
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            Entry::Native(app) => app.run(),
            #[cfg(all(
                feature = "designtime",
                any(target_os = "macos", target_os = "linux", target_os = "windows")
            ))]
            Entry::Development(launch) => launch
                .run()
                .map_err(|error| Error::message(format!("{error:#}"))),
        }
    }

    /// Mount an explicitly initialized native application in a caller-owned view.
    /// Use `init_with!` to ensure preparation happens in this process.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn mount(self, host: ViewHost<'_>) -> Result<MountedApp<'_>, Error> {
        match self.0 {
            Entry::Native(app) => app.mount(host),
            #[cfg(all(feature = "designtime", target_os = "macos"))]
            Entry::Development(_) => Err(Error::message("Embedding requires explicit in-process initialization: use pax_kit::init_with!(pax_kit::host::InitOptions::default())")),
        }
    }

    /// Run a native macOS application with explicit window options.
    #[cfg(target_os = "macos")]
    pub fn run_with(self, options: WindowOptions) -> Result<(), Error> {
        match self.0 {
            Entry::Native(app) => app.run_with(options),
            #[cfg(feature = "designtime")]
            Entry::Development(_) => Err(Error::message(
                "Window options require explicit in-process initialization with init_with!",
            )),
        }
    }
}

#[doc(hidden)]
pub fn __init_cartridge(
    recipe: &str,
    (options, project, application_role): (InitOptions, &'static str, bool),
) -> Result<Application, Error> {
    #[cfg(all(
        feature = "designtime",
        any(target_os = "macos", target_os = "linux", target_os = "windows")
    ))]
    if !application_role {
        let launch = pax_compiler::launcher::prepare(std::path::Path::new(project))
            .map_err(|error| Error::message(format!("{error:#}")))?;
        return Ok(Application(Entry::Development(launch)));
    }
    let _ = (project, application_role);
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        pax_host::__init_cartridge(recipe, options).map(|app| Application(Entry::Native(app)))
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        let _ = (recipe, options);
        Err(Error::message("Native hosting is unavailable on this platform. Enable designtime and use cargo run -- pax --target=web."))
    }
}
