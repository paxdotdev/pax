//! Native application entry and embedding through the Pax facade.
//!
//! `init!` and `init_with!` initialize in the calling process. They do not parse
//! application arguments or start build tools, development servers, or simulators.
//! Use `pax-cli` to package applications and run development sessions.

pub use pax_host::*;

/// A prepared native application in the calling process.
///
/// Initialization invokes the selected root initializer on the main thread.
/// Consume this value to run a standalone application. On Apple targets it can
/// also mount in a caller-owned host view; the Windows/Linux facade is standalone-only.
pub struct Application(Entry);

enum Entry {
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux"
    ))]
    Native(pax_host::PreparedApp),
}

impl Application {
    /// Enter the native application loop in the calling process.
    pub fn run(self) -> Result<(), Error> {
        match self.0 {
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "windows",
                target_os = "linux"
            ))]
            Entry::Native(app) => app.run(),
        }
    }

    /// Mount an explicitly initialized native application in a caller-owned view.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn mount(self, host: ViewHost<'_>) -> Result<MountedApp<'_>, Error> {
        match self.0 {
            Entry::Native(app) => app.mount(host),
        }
    }

    /// Run a native desktop application with explicit window options.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    pub fn run_with(self, options: WindowOptions) -> Result<(), Error> {
        match self.0 {
            Entry::Native(app) => app.run_with(options),
        }
    }
}

#[doc(hidden)]
pub fn __init_cartridge(recipe: &str, options: InitOptions) -> Result<Application, Error> {
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux"
    ))]
    {
        pax_host::__init_cartridge(recipe, options).map(|app| Application(Entry::Native(app)))
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux"
    )))]
    {
        let _ = (recipe, options);
        Err(Error::message("Native hosting is unavailable on this platform. Use pax-cli run --target web to build and serve the browser application."))
    }
}

#[doc(hidden)]
pub fn __init_cartridge_with_fonts(
    recipe: &str,
    options: InitOptions,
    fonts: &[(usize, &str, &str, &[u8])],
) -> Result<Application, Error> {
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux"
    ))]
    {
        pax_host::__init_cartridge_with_fonts(recipe, options, fonts)
            .map(|app| Application(Entry::Native(app)))
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
        target_os = "linux"
    )))]
    {
        let _ = fonts;
        __init_cartridge(recipe, options)
    }
}
