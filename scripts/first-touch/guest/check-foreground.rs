// Source-only CLI acceptance test. Compile with the guest's rustc.
use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
mod console {
    use super::*;
    use std::os::windows::process::CommandExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(pid: u32) -> i32;
        fn FreeConsole() -> i32;
        fn SetConsoleCtrlHandler(handler: usize, add: i32) -> i32;
        fn GenerateConsoleCtrlEvent(event: u32, group: u32) -> i32;
    }
    pub fn start(command: &mut Command) {
        command.creation_flags(0x10);
    } // CREATE_NEW_CONSOLE
    pub fn interrupt(pid: u32) {
        let status = Command::new(env::current_exe().unwrap())
            .args(["--interrupt", &pid.to_string()])
            .creation_flags(0x08000000)
            .status()
            .unwrap();
        assert!(status.success());
    }
    pub fn deliver(pid: u32) {
        unsafe {
            FreeConsole();
            assert_ne!(AttachConsole(pid), 0);
            assert_ne!(SetConsoleCtrlHandler(0, 1), 0);
            assert_ne!(GenerateConsoleCtrlEvent(0, 0), 0); // CTRL_C_EVENT to the isolated console
            thread::sleep(Duration::from_millis(250));
            FreeConsole();
        }
    }
}
#[cfg(unix)]
mod console {
    use super::*;
    pub fn start(_: &mut Command) {}
    pub fn interrupt(pid: u32) {
        assert!(Command::new("kill")
            .args(["-INT", &pid.to_string()])
            .status()
            .unwrap()
            .success());
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serving(port: u16) -> bool {
    let Ok(mut stream) =
        TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(1))
    else {
        return false;
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    if stream
        .write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut status = String::new();
    BufReader::new(stream).read_line(&mut status).is_ok()
        && status.split_whitespace().nth(1) == Some("200")
}

fn direct(project: &Path, temp: &Path, cli: &Path, port: u16, flags: &[&str]) {
    let log = temp.join("direct.log");
    let file = fs::File::create(&log).unwrap();
    let mut cmd = Command::new(cli);
    cmd.args(["run", "--target", "web", "--port", &port.to_string()])
        .args(flags)
        .current_dir(project)
        .stdout(file.try_clone().unwrap())
        .stderr(file)
        .stdin(Stdio::null());
    console::start(&mut cmd);
    let mut process = Process(cmd.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(600);
    while !serving(port) {
        assert!(
            process.0.try_wait().unwrap().is_none() && Instant::now() < deadline,
            "foreground start failed: {}",
            fs::read_to_string(&log).unwrap()
        );
        thread::sleep(Duration::from_millis(250));
    }
    console::interrupt(process.0.id());
    let deadline = Instant::now() + Duration::from_secs(12);
    while process.0.try_wait().unwrap().is_none()
        || TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], port))).is_ok()
    {
        assert!(
            Instant::now() < deadline,
            "Ctrl-C must release the CLI and port {port}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    #[cfg(windows)]
    if args.first().is_some_and(|s| s == "--interrupt") {
        console::deliver(args[1].parse().unwrap());
        return;
    }
    let project = PathBuf::from(&args[0]).canonicalize().unwrap();
    let cli = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "pax-cli".into());
    let temp = env::temp_dir().join(format!("pax-fg-smoke-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    if !cfg!(target_os = "macos") {
        let mut bg = Process(
            Command::new(&cli)
                .args(["run", "--bg"])
                .current_dir(&temp)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(8);
        let status = loop {
            if let Some(status) = bg.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "--bg did not reject promptly");
            thread::sleep(Duration::from_millis(40));
        };
        let mut diagnostic = String::new();
        bg.0.stderr
            .take()
            .unwrap()
            .read_to_string(&mut diagnostic)
            .unwrap();
        assert!(
            !status.success() && diagnostic.contains("--fg") && !temp.join(".pax").exists(),
            "{diagnostic}"
        );
        direct(&project, &temp, &cli, port, &[]);
    }
    direct(&project, &temp, &cli, port, &["--fg"]);
    fs::remove_dir_all(temp).unwrap();
    println!("Foreground Web run, Ctrl-C and port release passed.");
}
