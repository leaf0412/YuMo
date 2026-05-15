//! Custom-model worker invocation.
//!
//! Runs `custom_model_worker.py` as a one-shot child process per action,
//! independent of the long-running transcription daemon. This ensures
//! that dependency installs and model downloads cannot block ongoing
//! transcription requests.
//!
//! Protocol matches `custom_model_worker.py`:
//!   stdin  — a single JSON command line
//!   stdout — exactly one JSON object: {"ok": true, ...} or {"ok": false, "error": "..."}
//!   stderr — log lines and (on failure) python traceback
//!   exit   — 0 on success, non-zero on failure

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{AppError, AppResult};

pub type WorkerResponse = serde_json::Map<String, Value>;

/// Spawn the worker, send `cmd`, await its single JSON reply.
///
/// Errors are never swallowed: timeout, spawn failure, non-UTF8 / malformed
/// stdout, missing JSON, and `ok: false` replies all surface as `Err(...)`
/// with the worker's stderr included for diagnosis.
pub async fn run_action(
    python: &str,
    worker_script: &Path,
    cmd: Value,
    timeout: Duration,
) -> AppResult<WorkerResponse> {
    let action = cmd
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    log::info!(
        "[custom_worker] spawn python={} script={} action={}",
        python,
        worker_script.display(),
        action
    );

    let cmd_json = serde_json::to_string(&cmd)
        .map_err(|e| AppError::Transcription(format!("serialize worker cmd: {e}")))?;

    // Set cwd to the worker script's directory (i.e. data_dir / ~/.voiceink).
    // Some custom-model packages open files via relative paths like
    // "models/<id>/config.json" instead of honoring local_root; running with
    // cwd=data_dir makes those paths resolve against the actual download tree.
    let cwd = worker_script.parent().ok_or_else(|| {
        AppError::Transcription(format!(
            "worker script has no parent dir: {}",
            worker_script.display()
        ))
    })?;

    let mut child = Command::new(python)
        .arg(worker_script)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AppError::Transcription(format!(
                "spawn worker ({} {}): {}",
                python,
                worker_script.display(),
                e
            ))
        })?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Transcription("worker stdin handle missing".into()))?;
        stdin
            .write_all(cmd_json.as_bytes())
            .await
            .map_err(|e| AppError::Transcription(format!("write worker stdin: {e}")))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| AppError::Transcription(format!("write worker stdin: {e}")))?;
        // Dropping `stdin` here closes the pipe so the worker's
        // sys.stdin.readline() returns instead of blocking.
    }

    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            return Err(AppError::Transcription(format!(
                "worker wait failed: {e}"
            )))
        }
        Err(_) => {
            return Err(AppError::Transcription(format!(
                "worker timed out after {}s (action={})",
                timeout.as_secs(),
                action
            )))
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Worker contract: last non-empty stdout line is the JSON reply.
    // Be defensive against incidental warning prints earlier in stdout.
    let last_line = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|s| s.trim().to_string());

    let resp = match last_line {
        Some(line) => serde_json::from_str::<Value>(&line).map_err(|e| {
            AppError::Transcription(format!(
                "worker output not JSON: {e} — last_line='{}' stderr='{}'",
                line,
                stderr.trim()
            ))
        })?,
        None => {
            return Err(AppError::Transcription(format!(
                "worker produced no stdout (exit={:?}, stderr='{}')",
                output.status.code(),
                stderr.trim()
            )))
        }
    };

    let resp_obj = match resp {
        Value::Object(map) => map,
        other => {
            return Err(AppError::Transcription(format!(
                "worker output not a JSON object: {}",
                other
            )))
        }
    };

    let ok = resp_obj
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !ok {
        let err_msg = resp_obj
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("worker reported failure (no error message)");
        return Err(AppError::Transcription(format!(
            "{} (action={})",
            err_msg, action
        )));
    }

    if !output.status.success() {
        // Defensive: ok=true with non-zero exit should not happen; log loudly.
        log::warn!(
            "[custom_worker] ok=true but exit code {:?}; stderr='{}'",
            output.status.code(),
            stderr.trim()
        );
    }

    log::info!("[custom_worker] action={} completed", action);
    Ok(resp_obj)
}
