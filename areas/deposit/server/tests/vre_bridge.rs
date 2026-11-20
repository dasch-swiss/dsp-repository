//! `deposit-server` as the operator runs it: the `vre-bridge-refresh` subcommand's exit statuses,
//! and `serve` with the VRE bridge unconfigured, as in a public preview, and configured.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const KEY: &str = include_str!("../../vre-bridge/client/testdata/ed25519-private.pem");

/// The binary with an empty environment, in an empty directory, so no `editor.toml` and no
/// `EDITOR_*` or `PYROSCOPE_ENDPOINT` of the host reach it.
fn deposit_server(cwd: &Path) -> Command {
    // `CARGO_BIN_EXE_*` is runfiles-relative under Bazel; absolute, it does not depend on `cwd`.
    let mut command = Command::new(std::path::absolute(env!("CARGO_BIN_EXE_deposit_server")).unwrap());
    command.env_clear().env("RUST_LOG", "info").current_dir(cwd);
    command
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// The directories of an active bridge, and the variables that configure it.
struct Bridge {
    sync: TempDir,
    media: TempDir,
    work: TempDir,
    lock_dir: TempDir,
}

impl Bridge {
    fn new() -> Self {
        let temp = || tempfile::tempdir().unwrap();
        let bridge = Self { sync: temp(), media: temp(), work: temp(), lock_dir: temp() };
        std::fs::write(bridge.key_file(), KEY).unwrap();
        bridge
    }

    fn lock_file(&self) -> std::path::PathBuf {
        self.lock_dir.path().join("vre-bridge.lock")
    }

    fn key_file(&self) -> std::path::PathBuf {
        self.lock_dir.path().join("signing-key.pem")
    }

    fn configure<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("EDITOR_VRE_BRIDGE_SYNC_DIR", self.sync.path())
            .env("EDITOR_VRE_BRIDGE_MEDIA_DIR", self.media.path())
            .env("EDITOR_VRE_BRIDGE_WORK_DIR", self.work.path())
            .env("EDITOR_VRE_BRIDGE_LOCK_FILE", self.lock_file())
            .env("EDITOR_VRE_BRIDGE_ALLOWLIST", "0803=prod")
            // Nothing listens here; no test reaches a refresh's first request.
            .env("EDITOR_VRE_BRIDGE_SERVERS", "prod=https://prod.example.test")
            .env("EDITOR_VRE_BRIDGE_KEY_IDS", "prod=test-kid")
            .env(
                "EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES",
                format!("prod={}", self.key_file().display()),
            )
    }
}

// ---------------------------------------------------------------- vre-bridge-refresh

#[test]
fn test_refresh_without_configuration_exits_64_and_exports_nothing() {
    let cwd = tempfile::tempdir().unwrap();
    // An endpoint the test holds: an exporter would connect to it.
    let collector = TcpListener::bind("127.0.0.1:0").unwrap();
    collector.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", collector.local_addr().unwrap());
    let output = assert_cmd::Command::from_std(deposit_server(cwd.path()))
        .env("OTEL_EXPORTER_OTLP_ENDPOINT", &endpoint)
        .args(["vre-bridge-refresh", "0803"])
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();

    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(64), "{all}");
    assert!(all.contains("the VRE bridge is inactive"), "{all}");
    let connection = collector.accept();
    assert!(
        connection.as_ref().is_err_and(|e| e.kind() == std::io::ErrorKind::WouldBlock),
        "the subcommand connected to the OTLP endpoint: {connection:?}"
    );
}

#[test]
fn test_refresh_with_the_lock_held_exits_75() {
    let cwd = tempfile::tempdir().unwrap();
    let bridge = Bridge::new();
    let held = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(bridge.lock_file())
        .unwrap();
    held.try_lock().unwrap();

    let mut command = deposit_server(cwd.path());
    bridge.configure(&mut command).args(["vre-bridge-refresh", "0803"]);
    let output = assert_cmd::Command::from_std(command)
        .timeout(Duration::from_secs(30))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let all = format!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(75), "{all}");
    assert!(stdout.contains("the lock is held"), "{all}");
}

// ---------------------------------------------------------------- serve

/// A running `serve`, killed when dropped; its stdout and stderr lines arrive on `lines`.
struct Server {
    child: Child,
    lines: mpsc::Receiver<String>,
    seen: Vec<String>,
    port: u16,
    _cwd: TempDir,
}

impl Server {
    fn start(configure: impl FnOnce(&mut Command)) -> Self {
        let cwd = tempfile::tempdir().unwrap();
        let port = free_port();
        let mut command = deposit_server(cwd.path());
        command
            .arg("serve")
            .env("EDITOR_SITE_ADDR", format!("127.0.0.1:{port}"))
            .env("EDITOR_PUBLIC_DIR", cwd.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure(&mut command);
        let mut child = command.spawn().unwrap();
        let (sender, lines) = mpsc::channel();
        for stream in [
            Box::new(child.stdout.take().unwrap()) as Box<dyn Read + Send>,
            Box::new(child.stderr.take().unwrap()),
        ] {
            let sender = sender.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let _ = sender.send(line);
                }
            });
        }
        Self { child, lines, seen: Vec::new(), port, _cwd: cwd }
    }

    /// Whether `/healthz` answers 200 within the deadline.
    fn serves_healthz(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            if self.child.try_wait().unwrap().is_some() {
                return false;
            }
            if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
                stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let request = "GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
                let mut response = String::new();
                if stream.write_all(request.as_bytes()).is_ok() && stream.read_to_string(&mut response).is_ok() {
                    return response.starts_with("HTTP/1.1 200");
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    /// Whether a line containing `needle` is written within the deadline.
    fn wrote(&mut self, needle: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self.seen.iter().any(|line| line.contains(needle)) {
                return true;
            }
            match self.lines.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(line) => self.seen.push(line),
                Err(_) => return false,
            }
        }
    }

    fn output(&self) -> String {
        self.seen.join("\n")
    }

    /// Asserts that this process, and not another one on its port, serves `/healthz` and wrote
    /// `needle`.
    fn assert_serves_and_wrote(&mut self, needle: &str) {
        let healthy = self.serves_healthz();
        let listening = self.wrote(&format!("listening on http://127.0.0.1:{}", self.port));
        assert!(healthy && listening, "/healthz did not answer 200:\n{}", self.output());
        assert!(self.wrote(needle), "no line with {needle:?}:\n{}", self.output());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_serve_without_bridge_configuration_logs_it_inactive_and_serves() {
    Server::start(|_| {}).assert_serves_and_wrote("the VRE bridge is inactive");
}

#[test]
fn test_serve_with_bridge_configuration_builds_its_clients_and_serves() {
    // No `PYROSCOPE_ENDPOINT`: `serve` installs the TLS provider the bridge's clients need itself.
    let bridge = Bridge::new();
    Server::start(|command| {
        bridge.configure(command);
    })
    .assert_serves_and_wrote("the VRE bridge is active");
}

#[test]
fn test_serve_with_a_missing_signing_key_file_logs_the_bridge_inactive_and_serves() {
    let bridge = Bridge::new();
    std::fs::remove_file(bridge.key_file()).unwrap();
    let mut server = Server::start(|command| {
        bridge.configure(command);
    });
    server.assert_serves_and_wrote("the VRE bridge is inactive");
    assert!(server.wrote("EDITOR_VRE_BRIDGE_SIGNING_KEY_FILES"), "{}", server.output());
}
