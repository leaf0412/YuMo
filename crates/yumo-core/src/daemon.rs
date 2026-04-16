use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Generic event callback — replaces tauri::Emitter
// ---------------------------------------------------------------------------

/// Generic event callback for emitting status events to the frontend.
/// Signature: (event_name, payload_json).
pub type DaemonEventCallback = Box<dyn Fn(&str, &serde_json::Value) + Send + Sync>;

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

/// Maximum RSS (in bytes) for the Python daemon before auto-restart.
/// 8 GB — generous enough for large models, catches genuine leaks.
const DAEMON_RSS_LIMIT: u64 = 8 * 1024 * 1024 * 1024;

/// Query RSS of a process by pid (Unix only, uses `ps`).
#[cfg(unix)]
fn get_process_rss(pid: u32) -> Option<u64> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // ps reports RSS in kilobytes
    let kb: u64 = text.trim().parse().ok()?;
    Some(kb * 1024)
}

/// Stub for Windows — RSS check not yet implemented.
#[cfg(not(unix))]
fn get_process_rss(_pid: u32) -> Option<u64> {
    None
}

impl DaemonProcess {
    /// Write a JSON command and return the next meaningful response.
    ///
    /// "Meaningful" means: skip lines that start with `PROGRESS:` and skip
    /// intermediate `{"status":"downloading",...}` lines so callers only see
    /// terminal responses.
    fn send_command(&mut self, cmd: &Value) -> AppResult<DaemonResponse> {
        let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("unknown");
        log::info!("[daemon] sending command: {}", action);

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
            // Use read_response_raw() if you need downloading lines.
            if resp.status == "downloading" {
                continue;
            }

            log::info!("[daemon] response received, status={}", resp.status);
            return Ok(resp);
        }
    }

    /// Read the next single JSON response (including downloading status).
    /// Skips only bare PROGRESS: lines and empty lines.
    fn read_one_response(&mut self, timeout: std::time::Duration) -> AppResult<DaemonResponse> {
        let deadline = std::time::Instant::now() + timeout;
        let mut buf = String::new();
        loop {
            if std::time::Instant::now() >= deadline {
                return Err(AppError::Transcription(
                    format!("daemon response timed out after {}s", timeout.as_secs()),
                ));
            }
            buf.clear();
            let n = self.reader.read_line(&mut buf)
                .map_err(|e| AppError::Transcription(format!("read stdout: {e}")))?;
            if n == 0 {
                return Err(AppError::Transcription("daemon stdout closed unexpectedly".into()));
            }
            let trimmed = buf.trim();
            if trimmed.is_empty() || trimmed.starts_with("PROGRESS:") {
                continue;
            }
            let resp: DaemonResponse = serde_json::from_str(trimmed).map_err(|e| {
                AppError::Transcription(format!("parse daemon response: {e} — raw: {trimmed}"))
            })?;
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
    /// Lock-free status: true when daemon process is alive.
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Currently loaded model repo, separate from process lock.
    model_repo: Arc<Mutex<Option<String>>>,
}

impl DaemonManager {
    pub fn new(script_path: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            script_path,
            data_dir,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            model_repo: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if a python with required packages is available.
    pub fn has_python(&self) -> bool {
        has_working_python()
    }

    /// Verify python is available (no-op if already satisfied).
    pub fn ensure_python_static(_cb: Option<&DaemonEventCallback>) -> AppResult<()> {
        if has_working_python() {
            return Ok(());
        }
        Err(AppError::Transcription(
            "No working Python found. Please set the Python 3 path in Settings → System.".into(),
        ))
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
        log::info!("[daemon] using python: {}", python);
        log::info!("[daemon] [start] python={} script={}", python, self.script_path.display());

        let mut child = Command::new(&python)
            .arg(&self.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
        let stderr = child.stderr.take().expect("no stderr handle");

        // Forward daemon stderr lines to the unified log.
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) if !l.trim().is_empty() => log::info!("[daemon] [stderr] {}", l.trim()),
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        // Create the BufReader ONCE here — all subsequent reads use this same
        // reader so buffered data is never lost between calls.
        let mut reader = BufReader::new(stdout);

        // Wait for the ready handshake.
        if let Err(e) = wait_for_ready(&mut reader) {
            return Err(AppError::Transcription(
                format!("{e} (check log.txt for [daemon] [stderr] entries)")
            ));
        }

        *guard = Some(DaemonProcess {
            _child: child,
            stdin,
            reader,
            model_repo: None,
        });
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Send `{"action":"quit"}`, give the process a moment, then kill if still
    /// alive.
    pub fn stop(&self) {
        log::info!("[daemon] stop requested");
        let mut guard = match self.process.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(mut proc) = guard.take() {
            self.running.store(false, std::sync::atomic::Ordering::SeqCst);
            if let Ok(mut m) = self.model_repo.lock() { *m = None; }
            // Send quit without reading response (avoids blocking on read_line)
            let _ = proc.stdin.write_all(b"{\"action\":\"quit\"}\n");
            let _ = proc.stdin.flush();
            drop(proc.stdin); // close stdin to signal EOF

            // Give it up to 2 s to exit cleanly.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match proc._child.try_wait() {
                    Ok(Some(_)) => {
                        log::info!("[daemon] stopped cleanly");
                        break;
                    }
                    _ => {}
                }
                if std::time::Instant::now() >= deadline {
                    log::warn!("[daemon] force killing after timeout");
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
        let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("unknown");
        log::info!("[daemon] send_command: {}", action);

        let mut guard = self
            .process
            .lock()
            .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;

        let proc = guard.as_mut().ok_or_else(|| {
            AppError::Transcription("daemon is not running; call start() first".into())
        })?;

        proc.send_command(cmd)
    }

    /// Send a command and read responses one-by-one (including "downloading").
    /// Returns a channel of responses. The caller should read until a terminal
    /// status (not "downloading") is received.
    pub fn send_command_streaming(
        &self,
        cmd: &Value,
        timeout: std::time::Duration,
    ) -> AppResult<std::sync::mpsc::Receiver<DaemonResponse>> {
        let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("unknown");
        log::info!("[daemon] send_command_streaming: {}", action);

        let cmd_json = serde_json::to_string(cmd)
            .map_err(|e| AppError::Transcription(format!("serialize cmd: {e}")))?;

        // Write command
        {
            let mut guard = self.process.lock()
                .map_err(|_| AppError::Transcription("daemon mutex poisoned".into()))?;
            let proc = guard.as_mut().ok_or_else(|| {
                AppError::Transcription("daemon is not running; call start() first".into())
            })?;
            proc.stdin.write_all(cmd_json.as_bytes())
                .and_then(|_| proc.stdin.write_all(b"\n"))
                .and_then(|_| proc.stdin.flush())
                .map_err(|e| AppError::Transcription(format!("write stdin: {e}")))?;
        }

        // Spawn a thread that reads responses and sends them through a channel
        let (tx, rx) = std::sync::mpsc::channel();
        let process = self.process.clone();
        std::thread::spawn(move || {
            loop {
                let resp = {
                    let mut guard = match process.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    let proc = match guard.as_mut() {
                        Some(p) => p,
                        None => break,
                    };
                    match proc.read_one_response(timeout) {
                        Ok(r) => r,
                        Err(_) => break,
                    }
                };
                // "downloading" and "download_complete" are intermediate;
                // only "loaded", "success", "error" etc. are truly terminal.
                let is_terminal = resp.status != "downloading"
                    && resp.status != "download_complete";
                if tx.send(resp).is_err() {
                    break;
                }
                if is_terminal {
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// Async version of send_command — runs blocking IO on a dedicated thread
    /// with a configurable timeout. Does not block the tokio runtime.
    pub async fn send_command_async(
        &self,
        cmd: &Value,
        timeout: std::time::Duration,
    ) -> AppResult<DaemonResponse> {
        let action = cmd.get("action").and_then(|a| a.as_str()).unwrap_or("unknown");
        log::info!("[daemon] send_command_async: {}", action);

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
    // Memory watchdog
    // -----------------------------------------------------------------------

    /// Check daemon RSS and restart if it exceeds the limit.
    /// Call this after each transcription to catch leaks early.
    pub fn check_and_restart_if_bloated(&self) {
        let pid = {
            let guard = match self.process.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            match guard.as_ref() {
                Some(proc) => proc._child.id(),
                None => return,
            }
        };

        if let Some(rss) = get_process_rss(pid) {
            let rss_mb = rss / (1024 * 1024);
            log::info!("[daemon] post-transcription RSS: {} MB (limit: {} MB)", rss_mb, DAEMON_RSS_LIMIT / (1024 * 1024));

            if rss > DAEMON_RSS_LIMIT {
                log::warn!(
                    "[daemon] RSS {} MB exceeds limit {} MB — restarting daemon",
                    rss_mb,
                    DAEMON_RSS_LIMIT / (1024 * 1024)
                );
                self.stop();
                // Caller should re-start + re-load model on next transcription.
            }
        }
    }

    // -----------------------------------------------------------------------
    // Status accessors
    // -----------------------------------------------------------------------

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn loaded_model(&self) -> Option<String> {
        self.model_repo.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_loaded_model(&self, model: Option<String>) {
        if let Ok(mut guard) = self.model_repo.lock() {
            *guard = model;
        }
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// DaemonClient trait implementation
// ---------------------------------------------------------------------------

impl crate::daemon_client::DaemonClient for DaemonManager {
    fn send_command_async(
        &self,
        cmd: &serde_json::Value,
        timeout: std::time::Duration,
    ) -> impl std::future::Future<Output = Result<crate::daemon_client::DaemonResponse, AppError>> + Send {
        let fut = DaemonManager::send_command_async(self, cmd, timeout);
        async move {
            let resp = fut.await?;
            Ok(crate::daemon_client::DaemonResponse {
                status: resp.status,
                text: resp.text,
                error: resp.error,
                extra: resp.extra,
            })
        }
    }

    fn check_and_restart_if_bloated(&self) {
        DaemonManager::check_and_restart_if_bloated(self);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Quick check: is there a working python with required packages?
fn has_working_python() -> bool {
    if let Some(python) = resolve_python_path() {
        return std::path::Path::new(&python).exists() && python_has_deps(&python);
    }
    false
}

/// Resolve the python path: custom setting → `which python3`.
fn resolve_python_path() -> Option<String> {
    if let Some(custom) = read_custom_python_path() {
        if !custom.is_empty() && std::path::Path::new(&custom).exists() {
            return Some(custom);
        }
    }
    detect_system_python()
}

/// Check if a Python interpreter has the platform-appropriate deps installed.
#[cfg(target_os = "macos")]
fn python_has_deps(python: &str) -> bool {
    python_has_mlx(python)
}

#[cfg(not(target_os = "macos"))]
fn python_has_deps(python: &str) -> bool {
    python_has_transformers(python)
}

/// Check if a Python interpreter has `transformers` installed (for Windows/Linux).
#[cfg(not(target_os = "macos"))]
fn python_has_transformers(python: &str) -> bool {
    let output = std::process::Command::new(python)
        .args(["-c", "import transformers; print('ok')"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output();
    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "ok"
        }
        _ => false,
    }
}

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
                log::info!("[daemon] ready signal received");
                return Ok(());
            }
            // Valid JSON but not ready — log it
            log::info!("[daemon] [stdout-preready] {}", trimmed);
        } else {
            // Not valid JSON — log it (import warnings, etc.)
            log::info!("[daemon] [stdout-preready] {}", trimmed);
        }
    }
}

/// Minimum required mlx_audio version (Qwen3-ASR support requires ≥0.3.0)
const MIN_MLX_AUDIO_VERSION: &str = "0.3.0";

/// Check if a Python interpreter has mlx_audio installed with sufficient version.
fn python_has_mlx(python: &str) -> bool {
    let output = Command::new(python)
        .args(["-c", &format!(
            "from importlib.metadata import version; v = version('mlx-audio'); parts = [int(x) for x in v.split('.')[:3]]; min_parts = [int(x) for x in '{}'.split('.')]; ok = parts >= min_parts; print(f'{{v}} {{\"ok\" if ok else \"old\"}}')",
            MIN_MLX_AUDIO_VERSION
        )])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let is_ok = stdout.trim().ends_with("ok");
            if !is_ok {
                log::info!("[daemon] mlx_audio version too old: {} (need ≥{})", stdout.trim(), MIN_MLX_AUDIO_VERSION);
            }
            is_ok
        }
        _ => false,
    }
}


/// Path to the user-configured custom python path file.
fn custom_python_path_file() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".voiceink/python_path")
}

/// Read user-configured custom python path (if set and non-empty).
pub fn read_custom_python_path() -> Option<String> {
    let file = custom_python_path_file();
    let path = std::fs::read_to_string(&file).ok()?.trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Write user-configured custom python path.
/// Automatically resolves shims (asdf/mise) to the real binary path.
pub fn write_custom_python_path(path: &str) -> AppResult<()> {
    let resolved = resolve_real_python(path.trim());
    let file = custom_python_path_file();
    std::fs::write(&file, &resolved).map_err(|e| {
        AppError::Transcription(format!("failed to write python_path: {e}"))
    })?;
    log::info!("[daemon] custom python_path set to: {}", resolved);
    Ok(())
}

/// Detect system python3 path via `which python3`.
/// Automatically resolves shims to real binary paths.
pub fn detect_system_python() -> Option<String> {
    if let Ok(output) = Command::new("which").arg("python3").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            return Some(resolve_real_python(&path));
        }
    }
    None
}

/// Resolve a python path to the real binary (handles asdf/mise shims).
/// Runs `python -c "import sys; print(sys.executable)"` to get the actual interpreter.
fn resolve_real_python(python: &str) -> String {
    if let Ok(output) = Command::new(python)
        .args(["-c", "import sys; print(sys.executable)"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        if output.status.success() {
            let real = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !real.is_empty() && std::path::Path::new(&real).exists() {
                if real != python {
                    log::info!("[daemon] resolved python shim '{}' -> '{}'", python, real);
                }
                return real;
            }
        }
    }
    python.to_string()
}

/// Find python for the daemon: custom setting → `which python3`.
fn find_python() -> AppResult<String> {
    if let Some(python) = resolve_python_path() {
        if python_has_deps(&python) {
            log::info!("[daemon] using python: {}", python);
            return Ok(python);
        }
        log::warn!("[daemon] python '{}' found but missing required packages", python);
        return Err(AppError::Transcription(format!(
            "Python '{}' is missing required packages (mlx-audio). Install them or change the path in Settings → System.",
            python
        )));
    }
    Err(AppError::Transcription(
        "No Python 3 found. Please set the Python 3 path in Settings → System.".into(),
    ))
}
