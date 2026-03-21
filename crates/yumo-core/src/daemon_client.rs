// daemon_client.rs — minimal trait abstraction for the MLX daemon,
// so yumo-core modules don't depend on the concrete DaemonManager in src-tauri.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;

/// Subset of DaemonManager's response used by transcriber.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Async interface to the MLX daemon process.
/// Implemented by `DaemonManager` in src-tauri.
pub trait DaemonClient {
    fn send_command_async(
        &self,
        cmd: &Value,
        timeout: std::time::Duration,
    ) -> impl std::future::Future<Output = Result<DaemonResponse, AppError>> + Send;

    fn check_and_restart_if_bloated(&self);
}
