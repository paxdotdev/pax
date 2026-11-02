//! The adapter dependency supplies a macro containing copied file metadata.
//! No runtime Rust reference to its UI or engine is linked into this launcher.
use std::ffi::{c_char, c_void, CString};

#[cfg(unix)]
unsafe fn load(path: &std::path::Path) -> *mut c_void {
    extern "C" { fn dlopen(path: *const c_char, mode: i32) -> *mut c_void; }
    let path = CString::new(path.to_str().unwrap()).unwrap();
    dlopen(path.as_ptr(), 2)
}
#[cfg(unix)]
unsafe fn symbol(handle: *mut c_void, name: &str) -> *mut c_void {
    extern "C" { fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void; }
    dlsym(handle, CString::new(name).unwrap().as_ptr())
}
#[cfg(windows)]
unsafe fn load(path: &std::path::Path) -> *mut c_void {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" { fn LoadLibraryW(path: *const u16) -> *mut c_void; }
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    LoadLibraryW(path.as_ptr())
}
#[cfg(windows)]
unsafe fn symbol(handle: *mut c_void, name: &str) -> *mut c_void {
    #[link(name = "kernel32")]
    extern "system" { fn GetProcAddress(handle: *mut c_void, name: *const c_char) -> *mut c_void; }
    GetProcAddress(handle, CString::new(name).unwrap().as_ptr())
}

fn main() {
    let expected: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let root = std::env::current_exe().unwrap().parent().unwrap().to_owned();
    let name = boundary::cartridge_file!();
    let library = [root.join(name), root.join("deps").join(name)]
        .into_iter().find(|p| p.is_file()).expect("Cargo must build the adapter as a dependency");
    unsafe {
        let handle = load(&library);
        assert!(!handle.is_null(), "load adapter");
        macro_rules! get {
            ($name:literal, $ty:ty) => {{
                let address = symbol(handle, $name);
                assert!(!address.is_null(), "missing {}", $name);
                std::mem::transmute::<*mut c_void, $ty>(address)
            }};
        }
        let init = get!("pax_init_entry", unsafe extern "C" fn(*const u8, usize) -> *mut c_void);
        let tick = get!("pax_tick", unsafe extern "C" fn(*mut c_void, u64) -> u64);
        let free = get!("pax_dealloc_engine", unsafe extern "C" fn(*mut c_void));
        let count = get!("probe_count", extern "C" fn(usize) -> usize);
        let live = get!("probe_live", extern "C" fn() -> usize);
        assert_eq!((count(0), count(1), count(2), count(3), live()), (0, 0, 0, 0, 0));
        assert!(init(b"missing".as_ptr(), 7).is_null());
        assert_eq!(count(0), 0);
        let engine = init(std::ptr::null(), 0);
        assert!(!engine.is_null());
        assert_eq!(tick(engine, 1), expected + 1);
        assert_eq!((count(0), count(1), count(2), count(3), live()), (1, 0, 0, 0, 1));
        free(engine);
        assert_eq!(live(), 0);
        println!("PASS direct cargo run: selected root once; dependency roots never; owner freed engine");
    }
}
