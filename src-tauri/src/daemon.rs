use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

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
    /// Times out after `timeout` seconds (default 120s).
    fn read_response(&mut self) -> AppResult<DaemonResponse> {
        self.read_response_with_timeout(std::time::Duration::from_secs(120))
    }

    fn read_response_with_timeout(&mut self, timeout: std::time::Duration) -> AppResult<DaemonResponse> {
        let deadline = std::time::Instant::now() + timeout;
        let mut buf = String::new();
        loop {
            if std::time::Instant::now() >= deadline {
                return Err(AppError::Transcription(
                    format!("daemon response timed out after {}s", timeout.as_secs()),
                ));
            }

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
    process: Arc<Mutex<Option<DaemonProcess>>>,
    script_path: PathBuf,
    data_dir: PathBuf,
}

impl DaemonManager {
    pub fn new(script_path: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
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

    /// Async version of send_command — runs blocking IO on a dedicated thread
    /// with a configurable timeout. Does not block the tokio runtime.
    pub async fn send_command_async(
        &self,
        cmd: &Value,
        timeout: std::time::Duration,
    ) -> AppResult<DaemonResponse> {
        // Serialize command before moving into the blocking closure
        let cmd_json = serde_json::to_string(cmd)
            .map_err(|e| AppError::Transcription(format!("serialize cmd: {e}")))?;

        // Write command while holding the lock briefly
        {
            let mut guard = self
                .process
                .lock()
                .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;
            let proc = guard.as_mut().ok_or_else(|| {
                AppError::Transcription("daemon is not running; call start() first".into())
            })?;
            proc.stdin
                .write_all(cmd_json.as_bytes())
                .and_then(|_| proc.stdin.write_all(b"\n"))
                .and_then(|_| proc.stdin.flush())
                .map_err(|e| AppError::Transcription(format!("write stdin: {e}")))?;
        }

        // Read response on a blocking thread with timeout
        let process = self.process.clone();
        let result = tokio::time::timeout(timeout, tokio::task::spawn_blocking(move || {
            let mut guard = process
                .lock()
                .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;
            let proc = guard.as_mut().ok_or_else(|| {
                AppError::Transcription("daemon exited during transcription".into())
            })?;
            proc.read_response()
        }))
        .await;

        match result {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => Err(AppError::Transcription(format!("spawn_blocking failed: {e}"))),
            Err(_) => Err(AppError::Transcription(
                format!("daemon response timed out after {}s", timeout.as_secs()),
            )),
        }
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

/// Check if a Python interpreter has mlx_audio installed.
fn python_has_mlx(python: &str) -> bool {
    Command::new(python)
        .args(["-c", "import mlx_audio"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Collect candidate Python paths, matching VoiceInk's search order:
/// asdf → mise → miniforge → mambaforge → homebrew → system → local.
fn python_candidates() -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy().to_string();

        // asdf-managed pythons (newest first)
        let asdf_dir = format!("{home}/.asdf/installs/python");
        if let Ok(entries) = std::fs::read_dir(&asdf_dir) {
            let mut versions: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let bin = e.path().join("bin/python3");
                    bin.exists().then(|| bin.to_string_lossy().to_string())
                })
                .collect();
            versions.sort();
            versions.reverse();
            candidates.extend(versions);
        }

        // mise-managed pythons (newest first)
        let mise_dir = format!("{home}/.local/share/mise/installs/python");
        if let Ok(entries) = std::fs::read_dir(&mise_dir) {
            let mut versions: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let bin = e.path().join("bin/python3");
                    bin.exists().then(|| bin.to_string_lossy().to_string())
                })
                .collect();
            versions.sort();
            versions.reverse();
            candidates.extend(versions);
        }

        // conda environments
        candidates.push(format!("{home}/miniforge3/bin/python"));
        candidates.push(format!("{home}/mambaforge/bin/python"));
        candidates.push(format!("{home}/.local/bin/python3"));
    }

    // System-wide
    candidates.push("/opt/homebrew/bin/python3".into());
    candidates.push("/usr/local/bin/python3".into());
    candidates.push("/usr/bin/python3".into());

    candidates
}

/// Locate a Python 3 interpreter that has mlx_audio installed.
fn find_python() -> AppResult<String> {
    for candidate in python_candidates() {
        if std::path::Path::new(&candidate).exists() && python_has_mlx(&candidate) {
            return Ok(candidate);
        }
    }

    // Fallback: any working python3 (even without mlx_audio — will fail later
    // with a clearer error from the daemon itself).
    for candidate in python_candidates() {
        if std::path::Path::new(&candidate).exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::NotFound(
        "python3 not found; install Python 3 via Homebrew or python.org".into(),
    ))
}
