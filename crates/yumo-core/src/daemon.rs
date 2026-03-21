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

/// Query RSS of a process by pid (macOS only, uses `ps`).
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

    /// Check if a python with mlx_audio is available (venv or system).
    pub fn has_python(&self) -> bool {
        has_working_python()
    }

    /// Bootstrap venv if needed (blocking — call from spawn_blocking).
    pub fn ensure_python_static(cb: Option<&DaemonEventCallback>) -> AppResult<()> {
        if has_working_python() {
            return Ok(());
        }
        log::info!("[daemon] ensure_python: bootstrapping venv");
        bootstrap_venv(cb)?;
        Ok(())
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

/// Quick check: is there any python (venv or system) that has mlx_audio?
fn has_working_python() -> bool {
    let venv = venv_python_path();
    if std::path::Path::new(&venv).exists() && python_has_mlx(&venv) {
        return true;
    }
    for candidate in python_candidates() {
        if std::path::Path::new(&candidate).exists() && python_has_mlx(&candidate) {
            return true;
        }
    }
    false
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

/// Collect candidate Python paths, matching VoiceInk's search order:
/// asdf -> mise -> miniforge -> mambaforge -> homebrew -> system -> local.
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

    log::info!("[daemon] python candidates found: {}", candidates.len());
    candidates
}

/// Locate Python for the daemon — always uses the app-managed venv at ~/.voiceink/venv.
/// If the venv doesn't exist or has outdated deps, auto-bootstrap it.
/// Never uses system Python to avoid version conflicts and ensure reproducibility.
fn find_python() -> AppResult<String> {
    let venv_python = venv_python_path();

    // Fast path: venv exists and has correct mlx_audio version
    if std::path::Path::new(&venv_python).exists() && python_has_mlx(&venv_python) {
        log::info!("[daemon] using app venv python: {}", venv_python);
        return Ok(venv_python);
    }

    // Venv missing or outdated — bootstrap it
    if std::path::Path::new(&venv_python).exists() {
        log::info!("[daemon] venv python exists but mlx_audio outdated, upgrading...");
    } else {
        log::info!("[daemon] venv not found, bootstrapping...");
    }
    bootstrap_venv(None)
}

/// Path to the app-managed venv python binary.
fn venv_python_path() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".voiceink/venv/bin/python3")
        .to_string_lossy()
        .to_string()
}

/// Find any system python3 (doesn't need mlx_audio).
#[allow(dead_code)]
fn find_any_python() -> Option<String> {
    for candidate in python_candidates() {
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

/// Find or download the `uv` binary.
/// Checks ~/.voiceink/uv first, then system PATH, then auto-downloads.
fn find_uv() -> AppResult<PathBuf> {
    let data_uv = dirs::home_dir()
        .unwrap_or_default()
        .join(".voiceink/uv");
    if data_uv.exists() {
        return Ok(data_uv);
    }
    // System uv as fallback
    if let Ok(output) = Command::new("which").arg("uv").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            return Ok(PathBuf::from(path));
        }
    }
    // Auto-download uv to ~/.voiceink/uv
    log::info!("[daemon] uv not found, downloading...");
    download_uv(&data_uv)?;
    Ok(data_uv)
}

/// Download the `uv` binary via the official install script.
fn download_uv(dest: &std::path::Path) -> AppResult<()> {
    let parent = dest.parent().unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| AppError::Io(format!("create dir failed: {e}")))?;

    // Use the official standalone installer, extract to a temp location then move
    let tmp_dir = parent.join(".uv_install_tmp");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| AppError::Io(format!("create tmp dir failed: {e}")))?;

    let status = Command::new("sh")
        .args(["-c", &format!(
            "curl -fsSL https://astral.sh/uv/install.sh | CARGO_HOME='{}' sh",
            tmp_dir.to_string_lossy()
        )])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| AppError::Io(format!("uv download failed: {e}")))?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(AppError::Io("uv install script failed".into()));
    }

    // The installer puts uv at CARGO_HOME/bin/uv
    let installed = tmp_dir.join("bin/uv");
    if installed.exists() {
        std::fs::copy(&installed, dest)
            .map_err(|e| AppError::Io(format!("copy uv binary failed: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
        }
        log::info!("[daemon] uv downloaded to {:?}", dest);
    } else {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(AppError::Io("uv binary not found after install".into()));
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(())
}

/// Run a command, streaming its stderr to log.txt line by line.
/// Splits on both \r and \n to handle uv's carriage-return progress bars.
fn run_and_stream_stderr(cmd: &mut Command, tag: &str) -> AppResult<()> {
    use std::io::Read;

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Transcription(format!("{tag} spawn failed: {e}")))?;

    let stderr = child.stderr.take().expect("no stderr");
    let mut buf = Vec::new();

    for byte in stderr.bytes() {
        match byte {
            Ok(b'\n') | Ok(b'\r') => {
                if !buf.is_empty() {
                    let line = String::from_utf8_lossy(&buf);
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        log::info!("[daemon] [bootstrap] [{}] {}", tag, trimmed);
                    }
                    buf.clear();
                }
            }
            Ok(b) => buf.push(b),
            Err(_) => break,
        }
    }
    // Flush remaining
    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            log::info!("[daemon] [bootstrap] [{}] {}", tag, trimmed);
        }
    }

    let status = child.wait()
        .map_err(|e| AppError::Transcription(format!("{tag} wait failed: {e}")))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(AppError::Transcription(format!("{tag} failed exit_code={code}")));
    }
    Ok(())
}

/// Create a venv at ~/.voiceink/venv and install mlx-audio-plus using `uv`.
/// `uv` is auto-downloaded on first use if not present.
/// Returns the venv python path on success.
fn bootstrap_venv(cb: Option<&DaemonEventCallback>) -> AppResult<String> {
    let start = std::time::Instant::now();
    let uv = find_uv()?;
    log::info!("[daemon] [bootstrap] using uv: {:?}", uv);

    let venv_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".voiceink/venv");
    let venv_dir_str = venv_dir.to_string_lossy().to_string();

    // Step 1: Create venv
    log::info!("[daemon] [bootstrap] creating venv at {} python=3.12", venv_dir_str);
    if let Some(cb) = cb {
        cb("daemon-setup-status", &serde_json::json!({
            "stage": "creating_venv"
        }));
    }
    run_and_stream_stderr(
        Command::new(&uv).args(["venv", &venv_dir_str, "--python", "3.12"]),
        "uv-venv",
    ).map_err(|e| {
        log::error!("[daemon] [bootstrap] FAILED stage=venv elapsed_ms={}", start.elapsed().as_millis());
        e
    })?;

    // Step 2: Install deps
    log::info!("[daemon] [bootstrap] venv created, installing deps: mlx-audio-plus, soundfile");
    if let Some(cb) = cb {
        cb("daemon-setup-status", &serde_json::json!({
            "stage": "installing_deps",
            "message": "正在安装 Python 依赖，首次需要几分钟..."
        }));
    }
    run_and_stream_stderr(
        Command::new(&uv).args([
            "pip", "install",
            "--python", &format!("{}/bin/python3", venv_dir_str),
            "--upgrade", "mlx-audio", "mlx-audio-plus", "soundfile", "httpx[socks]",
        ]),
        "uv-pip",
    ).map_err(|e| {
        log::error!("[daemon] [bootstrap] FAILED stage=pip_install elapsed_ms={}", start.elapsed().as_millis());
        e
    })?;

    let venv_python = venv_python_path();
    log::info!("[daemon] [bootstrap] complete elapsed_ms={}", start.elapsed().as_millis());
    Ok(venv_python)
}
