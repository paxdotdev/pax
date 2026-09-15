use pax_message::http_api::{
    CommandFamily, CommandOutcome, TelemetryEvent, TelemetryRequest, CLI_LATEST_RELEASE_PATH,
    CLI_TELEMETRY_PATH,
};
use serde_json::json;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Once};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const WORKER_FLAG: &str = "--pax-telemetry-worker";
const FOREGROUND_BUDGET: Duration = Duration::from_millis(900);

struct Cli {
    home: tempfile::TempDir,
    api_base: String,
}

impl Cli {
    fn new(server: &CaptureServer) -> Self {
        // macOS can assess a freshly linked executable before dyld starts it.
        // Separate that cold-launch cost from the HTTP-response exit tests.
        static WARM_EXECUTABLE: Once = Once::new();
        WARM_EXECUTABLE.call_once(|| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_pax-cli"));
            command.arg("--version");
            assert_success(&RunningCli::spawn(command, None).finish(Duration::from_secs(15)));
        });
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join("project")).unwrap();
        Self {
            home,
            api_base: server.url.clone(),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pax-cli"));
        command
            .args(args)
            .current_dir(self.home.path().join("project"))
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("LOCALAPPDATA", self.home.path().join("local"))
            .env("APPDATA", self.home.path().join("roaming"))
            .env("XDG_STATE_HOME", self.home.path().join("state"))
            .env("PAX_API_BASE_URL", &self.api_base)
            .env_remove("CI")
            .env_remove("PAX_TELEMETRY")
            .env_remove("DO_NOT_TRACK")
            .env_remove("DNT");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        RunningCli::spawn(self.command(args), None).finish(FOREGROUND_BUDGET)
    }

    fn initialize(&self, server: &CaptureServer) {
        let output = self.run(&["clean"]);
        assert_success(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("This command sends no telemetry"));
        server.assert_no_posts(Duration::from_millis(200));
    }

    fn capture_identity(&self, server: &CaptureServer) -> String {
        assert_success(&self.run(&["clean"]));
        let captured = server.next_post();
        let request = captured.telemetry();
        captured.reply();
        request.installation_id
    }
}

// Read both pipes independently so the deadline also detects a worker that
// accidentally inherits stdout/stderr after its parent has already exited.
struct RunningCli {
    child: Child,
    stdout: mpsc::Receiver<io::Result<Vec<u8>>>,
    stderr: mpsc::Receiver<io::Result<Vec<u8>>>,
}

impl RunningCli {
    fn start(mut command: Command, keep_stdin_open: bool) -> Self {
        command
            .stdin(if keep_stdin_open {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let stdout = read_pipe(child.stdout.take().unwrap());
        let stderr = read_pipe(child.stderr.take().unwrap());
        Self {
            child,
            stdout,
            stderr,
        }
    }

    fn spawn(command: Command, input: Option<&[u8]>) -> Self {
        let mut running = Self::start(command, input.is_some());
        if let Some(input) = input {
            let mut stdin = running.child.stdin.take().unwrap();
            // Rejecting an oversized worker message can close stdin early.
            let _ = stdin.write_all(input);
        }
        running
    }

    fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().unwrap()
    }

    #[track_caller]
    fn finish(mut self, budget: Duration) -> Output {
        let deadline = Instant::now() + budget;
        let status = loop {
            if let Some(status) = self.exited() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "CLI waited for background networking"
            );
            thread::sleep(Duration::from_millis(2));
        };
        // A reader thread still needs to observe EOF and forward its bytes
        // after process exit. Give that scheduling its own small bound; the
        // server response remains gated throughout, so inherited pipes fail.
        let pipe_deadline = Instant::now() + budget.min(Duration::from_millis(500));
        let stdout = self
            .stdout
            .recv_timeout(pipe_deadline.saturating_duration_since(Instant::now()))
            .expect("stdout stayed open after the foreground CLI exited")
            .unwrap();
        let stderr = self
            .stderr
            .recv_timeout(pipe_deadline.saturating_duration_since(Instant::now()))
            .expect("stderr stayed open after the foreground CLI exited")
            .unwrap();
        Output {
            status,
            stdout,
            stderr,
        }
    }
}

impl Drop for RunningCli {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}

struct CapturedRequest {
    method: String,
    path: String,
    body: Vec<u8>,
    stream: TcpStream,
}

impl CapturedRequest {
    fn telemetry(&self) -> TelemetryRequest {
        assert_eq!(self.method, "POST");
        assert_eq!(self.path, CLI_TELEMETRY_PATH);
        serde_json::from_slice(&self.body).expect("the worker must send the closed shared DTO")
    }

    fn reply(mut self) {
        let response = if self.method == "GET" {
            let body = r#"{"latest_version":"0.0.0"}"#;
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
        } else {
            "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
        };
        // An update request may be abandoned when the foreground exits.
        let _ = self.stream.write_all(response.as_bytes());
    }
}

struct CaptureServer {
    url: String,
    requests: mpsc::Receiver<CapturedRequest>,
    stop: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
}

impl CaptureServer {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (sender, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let listener = thread::spawn(move || {
            let mut readers = Vec::new();
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let sender = sender.clone();
                        readers.push(thread::spawn(move || {
                            if let Ok(request) = read_request(stream) {
                                let _ = sender.send(request);
                            }
                        }));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("loopback accept failed: {error}"),
                }
            }
            for reader in readers {
                reader.join().unwrap();
            }
        });
        Self {
            url,
            requests,
            stop,
            listener: Some(listener),
        }
    }

    #[track_caller]
    fn next_post(&self) -> CapturedRequest {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let request = self
                .requests
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("detached worker did not deliver telemetry after its parent exited");
            if request.method == "POST" {
                return request;
            }
            assert_eq!(request.path, CLI_LATEST_RELEASE_PATH);
            request.reply();
        }
    }

    fn assert_no_posts(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while let Ok(request) = self
            .requests
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            assert_ne!(request.method, "POST", "unexpected telemetry request");
            assert_eq!(request.path, CLI_LATEST_RELEASE_PATH);
            request.reply();
        }
    }

    fn assert_no_requests(&self) {
        assert!(
            self.requests
                .recv_timeout(Duration::from_millis(200))
                .is_err(),
            "management/private worker made a network request"
        );
    }
}

impl Drop for CaptureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.listener.take().unwrap().join().unwrap();
    }
}

fn read_request(stream: TcpStream) -> io::Result<CapturedRequest> {
    // Accepted sockets can inherit the listener's nonblocking mode (notably
    // macOS/Windows). Read timeouts do not clear it; reset it before parsing.
    stream.set_nonblocking(false)?;
    // The capture server must not abandon a connection sooner than the CLI's
    // own bounded request. Scheduling under load can delay its first bytes.
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_millis(500)))?;
    let mut reader = BufReader::new(stream);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| io::Error::other("missing method"))?
        .to_owned();
    let path = parts
        .next()
        .ok_or_else(|| io::Error::other("missing path"))?
        .to_owned();
    let mut content_length = 0;
    let mut header_bytes = first_line.len();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(io::Error::other("request closed before its headers"));
        }
        header_bytes += line.len();
        if header_bytes > 8192 {
            return Err(io::Error::other("oversized test request"));
        }
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().map_err(io::Error::other)?;
            }
        }
    }
    if content_length > 8192 {
        return Err(io::Error::other("oversized test body"));
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    Ok(CapturedRequest {
        method,
        path,
        body,
        stream: reader.into_inner(),
    })
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn worker_message(id: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "installation_id": id,
        "event": TelemetryEvent::CommandOutcome {
            command: CommandFamily::Clean,
            target: None,
            outcome: CommandOutcome::Succeeded,
        },
    }))
    .unwrap()
}

#[test]
fn first_notice_and_off_allow_opting_out_without_telemetry() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    cli.initialize(&server);
    assert_success(&cli.run(&["telemetry", "off"]));
    assert_success(&cli.run(&["telemetry", "status"]));
    server.assert_no_requests();

    let untouched_server = CaptureServer::new();
    let untouched = Cli::new(&untouched_server);
    assert_success(&untouched.run(&["telemetry", "off"]));
    untouched_server.assert_no_requests();
}

#[test]
fn detached_delivery_survives_parent_exit_without_inheriting_output_pipes() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    cli.initialize(&server);

    // No server response is sent until the foreground and both output pipes
    // have finished. An in-process sender or inherited pipe cannot pass this.
    let output = cli.run(&["clean"]);
    assert_success(&output);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("This command sends no telemetry"));
    let captured = server.next_post();
    let first = captured.telemetry();
    assert!(uuid::Uuid::parse_str(&first.installation_id).is_ok());
    assert_eq!(first.cli_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(
        first.event,
        TelemetryEvent::CommandOutcome {
            command: CommandFamily::Clean,
            target: None,
            outcome: CommandOutcome::Succeeded,
        }
    );
    captured.reply();

    let second_id = cli.capture_identity(&server);
    assert_eq!(first.installation_id, second_id);
}

#[test]
fn in_flight_delivery_does_not_lock_foreground_state_and_off_revokes_before_waiting() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    cli.initialize(&server);
    assert_success(&cli.run(&["clean"]));
    let in_flight = server.next_post();

    let status = RunningCli::spawn(cli.command(&["telemetry", "status"]), None)
        .finish(Duration::from_millis(400));
    assert_success(&status);
    assert!(String::from_utf8_lossy(&status.stdout).contains("telemetry is on"));
    assert_success(
        &RunningCli::spawn(cli.command(&["clean"]), None).finish(Duration::from_millis(400)),
    );
    let second_in_flight = server.next_post();

    let mut off = RunningCli::spawn(cli.command(&["telemetry", "off"]), None);
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        let status = cli.run(&["telemetry", "status"]);
        assert_success(&status);
        if String::from_utf8_lossy(&status.stdout).contains("telemetry is off") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "opt-out waited for HTTP before persisting revocation"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        off.exited().is_none(),
        "opt-out returned while an authorized HTTP delivery was still in flight"
    );
    in_flight.reply();
    second_in_flight.reply();
    assert_success(&off.finish(FOREGROUND_BUDGET));
    server.assert_no_posts(Duration::from_millis(200));
}

#[test]
fn private_worker_rechecks_opt_out_and_environment_and_rejects_bad_input_silently() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    cli.initialize(&server);
    let id = cli.capture_identity(&server);
    server.assert_no_posts(Duration::from_millis(100));
    let message = worker_message(&id);

    for (name, value) in [
        ("PAX_TELEMETRY", "off"),
        ("DO_NOT_TRACK", "1"),
        ("CI", "true"),
    ] {
        let mut command = cli.command(&[WORKER_FLAG]);
        command.env(name, value);
        let output = RunningCli::spawn(command, Some(&message)).finish(FOREGROUND_BUDGET);
        assert_success(&output);
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    server.assert_no_requests();

    for input in [
        Vec::new(),
        b"not json".to_vec(),
        worker_message("not-a-uuid"),
        serde_json::to_vec(&json!({"installation_id": id, "event": {"type": "unknown"}})).unwrap(),
        serde_json::to_vec(&json!({"installation_id": id, "event": {"type": "run_ready", "target": "web"}, "extra": "rejected"})).unwrap(),
        vec![b'x'; 16_384],
    ] {
        let output = RunningCli::spawn(cli.command(&[WORKER_FLAG]), Some(&input)).finish(FOREGROUND_BUDGET);
        assert_success(&output);
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    server.assert_no_requests();

    assert_success(&cli.run(&["telemetry", "off"]));
    let output =
        RunningCli::spawn(cli.command(&[WORKER_FLAG]), Some(&message)).finish(FOREGROUND_BUDGET);
    assert_success(&output);
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    server.assert_no_requests();
}

#[test]
fn delayed_update_response_never_holds_foreground_exit() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    // Suppress telemetry so this test isolates the independent update thread.
    let mut command = cli.command(&["clean"]);
    command.env("PAX_TELEMETRY", "off");
    let mut running = RunningCli::spawn(command, None);
    let deadline = Instant::now() + FOREGROUND_BUDGET;
    let held_get = loop {
        if let Ok(request) = server.requests.recv_timeout(Duration::from_millis(2)) {
            assert_eq!(request.method, "GET");
            assert_eq!(request.path, CLI_LATEST_RELEASE_PATH);
            break Some(request);
        }
        if running.exited().is_some() {
            break None;
        }
        assert!(
            Instant::now() < deadline,
            "CLI did not finish or start its update request"
        );
    };
    // Measure after a request arrives, avoiding cold process-start latency.
    assert_success(&running.finish(Duration::from_millis(150)));
    drop(held_get);
}

#[test]
fn private_worker_has_a_deadline_even_when_stdin_never_closes() {
    let server = CaptureServer::new();
    let cli = Cli::new(&server);
    let mut worker = RunningCli::start(cli.command(&[WORKER_FLAG]), true);
    thread::sleep(Duration::from_millis(100));
    assert!(
        worker.exited().is_none(),
        "worker did not wait for its stdin message"
    );
    // This handle deliberately retains the write end until the worker exits.
    let output = worker.finish(Duration::from_secs(7));
    assert_success(&output);
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    server.assert_no_requests();
}
