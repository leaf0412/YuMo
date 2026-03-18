use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Response envelope sent by the Python daemon on stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<serde_json::Number>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

// ---------------------------------------------------------------------------
// Internal process handle
// ---------------------------------------------------------------------------

struct DaemonProcess {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    model_repo: Option<String>,
}

impl DaemonProcess {
    /// Write a JSON command and return the next meaningful response.
    ///
    /// "Meaningful" means: skip lines that start with `PROGRESS:` and skip
    /// intermediate `{"status":"downloading",...}` lines so callers only see
    /// terminal responses.
    fn send_command(&mut self, cmd: &Value) -> AppResult<DaemonResponse> {
        let line = serde_json::to_string(cmd)
            .map_err(|e| AppError::Transcription(format!("serialize cmd: {e}")))?;

        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| AppError::Transcription(format!("write stdin: {e}")))?;

        self.read_response()
    }

    /// Read lines until a terminal (non-progress, non-downloading) response.
    fn read_response(&mut self) -> AppResult<DaemonResponse> {
        let mut buf = String::new();
        loop {
            buf.clear();
            let n = self
                .reader
                .read_line(&mut buf)
                .map_err(|e| AppError::Transcription(format!("read stdout: {e}")))?;

            if n == 0 {
                return Err(AppError::Transcription(
                    "daemon stdout closed unexpectedly".into(),
                ));
            }

            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip bare progress log lines emitted by some libraries.
            if trimmed.starts_with("PROGRESS:") {
                continue;
            }

            let resp: DaemonResponse = serde_json::from_str(trimmed).map_err(|e| {
                AppError::Transcription(format!("parse daemon response: {e} — raw: {trimmed}"))
            })?;

            // Skip intermediate downloading status lines; keep reading.
            if resp.status == "downloading" {
                continue;
            }

            return Ok(resp);
        }
    }
}

// ---------------------------------------------------------------------------
// Public manager
// ---------------------------------------------------------------------------

pub struct DaemonManager {
    process: Mutex<Option<DaemonProcess>>,
    script_path: PathBuf,
    data_dir: PathBuf,
}

impl DaemonManager {
    pub fn new(script_path: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            process: Mutex::new(None),
            script_path,
            data_dir,
        }
    }

    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Spawn the Python daemon, wait for `{"status":"ready"}`, and store the
    /// process handle.  If a daemon is already running this is a no-op.
    pub fn start(&self) -> AppResult<()> {
        let mut guard = self
            .process
            .lock()
            .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;

        if guard.is_some() {
            return Ok(());
        }

        let python = find_python()?;
        let stderr_path = self.data_dir.join("daemon_stderr.log");
        let stderr_file = std::fs::File::create(&stderr_path)
            .map_err(|e| AppError::Io(format!("create stderr log: {e}")))?;

        let mut child = Command::new(&python)
            .arg(&self.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr_file)
            .spawn()
            .map_err(|e| {
                AppError::Transcription(format!(
                    "spawn daemon ({python} {}): {e}",
                    self.script_path.display()
                ))
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Transcription("no stdin handle".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Transcription("no stdout handle".into()))?;

        // Create the BufReader ONCE here — all subsequent reads use this same
        // reader so buffered data is never lost between calls.
        let mut reader = BufReader::new(stdout);

        // Wait for the ready handshake.
        wait_for_ready(&mut reader)?;

        *guard = Some(DaemonProcess {
            _child: child,
            stdin,
            reader,
            model_repo: None,
        });

        Ok(())
    }

    /// Send `{"action":"quit"}`, give the process a moment, then kill if still
    /// alive.
    pub fn stop(&self) {
        let mut guard = match self.process.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(mut proc) = guard.take() {
            // Send quit without reading response (avoids blocking on read_line)
            let _ = proc.stdin.write_all(b"{\"action\":\"quit\"}\n");
            let _ = proc.stdin.flush();
            drop(proc.stdin); // close stdin to signal EOF

            // Give it up to 2 s to exit cleanly.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match proc._child.try_wait() {
                    Ok(Some(_)) => break,
                    _ => {}
                }
                if std::time::Instant::now() >= deadline {
                    let _ = proc._child.kill();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    // -----------------------------------------------------------------------
    // IPC
    // -----------------------------------------------------------------------

    /// Write a JSON command to the daemon's stdin and return the terminal
    /// response.  Skips PROGRESS: lines and intermediate "downloading" lines.
    pub fn send_command(&self, cmd: &Value) -> AppResult<DaemonResponse> {
        let mut guard = self
            .process
            .lock()
            .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;

        let proc = guard.as_mut().ok_or_else(|| {
            AppError::Transcription("daemon is not running; call start() first".into())
        })?;

        proc.send_command(cmd)
    }

    // -----------------------------------------------------------------------
    // Status accessors
    // -----------------------------------------------------------------------

    pub fn is_running(&self) -> bool {
        self.process
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub fn loaded_model(&self) -> Option<String> {
        self.process
            .lock()
            .ok()
            .and_then(|g| g.as_ref().and_then(|p| p.model_repo.clone()))
    }

    pub fn set_loaded_model(&self, model: Option<String>) {
        if let Ok(mut guard) = self.process.lock() {
            if let Some(proc) = guard.as_mut() {
                proc.model_repo = model;
            }
        }
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Block on the reader until `{"status":"ready"}` arrives.
fn wait_for_ready(reader: &mut BufReader<ChildStdout>) -> AppResult<()> {
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .map_err(|e| AppError::Transcription(format!("read ready line: {e}")))?;

        if n == 0 {
            return Err(AppError::Transcription(
                "daemon exited before sending ready".into(),
            ));
        }

        let trimmed = buf.trim();
        if trimmed.is_empty() || trimmed.starts_with("PROGRESS:") {
            continue;
        }

        // Try to parse as JSON; if it's the ready signal we're done.
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            if v.get("status").and_then(|s| s.as_str()) == Some("ready") {
                return Ok(());
            }
        }
        // Any other output before ready is silently ignored.
    }
}

/// Locate a Python 3 interpreter, trying `which python3` first then known
/// fixed paths on macOS.
fn find_python() -> AppResult<String> {
    // 1. Try the shell PATH.
    if let Ok(out) = Command::new("which").arg("python3").output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    // 2. Fall back to well-known macOS locations.
    for p in &["/opt/homebrew/bin/python3", "/usr/local/bin/python3"] {
        if std::path::Path::new(p).exists() {
            return Ok(p.to_string());
        }
    }

    Err(AppError::NotFound(
        "python3 not found; install Python 3 via Homebrew or python.org".into(),
    ))
}
