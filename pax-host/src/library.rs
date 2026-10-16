//! Process-pinned native libraries. Callback owners may outlive an application,
//! so neither a successful load nor a partially resolved ABI is unloaded.
use std::{ffi::c_void, path::Path};

#[cfg(unix)]
#[cfg_attr(target_os = "linux", link(name = "dl"))]
extern "C" {
    fn dlopen(path: *const std::ffi::c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const std::ffi::c_char) -> *mut c_void;
    fn dlerror() -> *const std::ffi::c_char;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(handle: *mut c_void, name: *const u8) -> *mut c_void;
}

pub(crate) unsafe fn open(path: &Path) -> Result<*mut c_void, String> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|error| error.to_string())?;
        let handle = dlopen(path.as_ptr(), 2); // RTLD_NOW; symbols remain local.
        if handle.is_null() {
            let error = dlerror();
            return Err(if error.is_null() {
                "Unable to load PaxHost".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            });
        }
        Ok(handle)
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let mut path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if path.contains(&0) {
            return Err("Native library path contains a NUL character".into());
        }
        path.push(0);
        // Dependencies are resolved beside this SDK and in system directories,
        // rather than through the caller's working directory.
        let handle = LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x100 | 0x1000);
        if handle.is_null() {
            return Err(format!(
                "Unable to load PaxHost: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(handle)
    }
}

pub(crate) unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &[u8]) -> Result<T, String> {
    #[cfg(unix)]
    let pointer = dlsym(handle, name.as_ptr().cast());
    #[cfg(target_os = "windows")]
    let pointer = GetProcAddress(handle, name.as_ptr());
    if pointer.is_null() {
        return Err(format!(
            "Missing native symbol {}",
            String::from_utf8_lossy(name)
        ));
    }
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of_val(&pointer));
    Ok(std::mem::transmute_copy(&pointer))
}
