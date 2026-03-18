use rusqlite::Connection;
use std::sync::Mutex;

use crate::pipeline::PipelineState;
use crate::recorder::RecordingHandle;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub pipeline_state: Mutex<PipelineState>,
    pub recording_handle: Mutex<Option<RecordingHandle>>,
    pub models_dir: std::path::PathBuf,
    pub daemon: crate::daemon::DaemonManager,
}

impl AppState {
    pub fn new(conn: Connection, models_dir: std::path::PathBuf, daemon: crate::daemon::DaemonManager) -> Self {
        Self {
            db: Mutex::new(conn),
            pipeline_state: Mutex::new(PipelineState::Idle),
            recording_handle: Mutex::new(None),
            models_dir,
            daemon,
        }
    }
}
