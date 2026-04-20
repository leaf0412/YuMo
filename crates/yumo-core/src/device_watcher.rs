//! Cross-platform audio input device watcher.
//!
//! Polls `platform::recorder::list_input_devices()` on a background thread and
//! invokes a caller-supplied callback whenever the device set or the system
//! default device changes. Works identically on macOS, Linux, and Windows —
//! avoids the per-platform native API maze (CoreAudio property listeners on
//! macOS, IMMNotificationClient on Windows, PipeWire/PulseAudio/ALSA on Linux).

use crate::platform::{recorder, AudioInputDevice};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const POLL_INTERVAL_MS: u64 = 1000;

/// Compact signature of a device list — equal iff the device set and the
/// system-default device are both unchanged. Order-independent.
pub fn snapshot(devices: &[AudioInputDevice]) -> (Vec<u32>, Option<u32>) {
    let mut ids: Vec<u32> = devices.iter().map(|d| d.id).collect();
    ids.sort_unstable();
    let default_id = devices.iter().find(|d| d.is_default).map(|d| d.id);
    (ids, default_id)
}

/// Returned by [`start`]. Drop or call [`WatcherHandle::stop`] to shut down.
/// In typical app code this is `mem::forget`-ed and runs for the process lifetime.
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
}

impl WatcherHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawn a background polling thread. `initial` seeds the comparison snapshot
/// (typically the device list cached at startup), so the first real change is
/// the first invocation of `on_change`.
///
/// `on_change` is called on the polling thread with the new device list.
/// Keep it cheap or spawn your own worker — a slow callback delays the next poll.
pub fn start<F>(initial: Vec<AudioInputDevice>, on_change: F) -> WatcherHandle
where
    F: Fn(Vec<AudioInputDevice>) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let mut last = snapshot(&initial);

    std::thread::Builder::new()
        .name("device-watcher".into())
        .spawn(move || {
            info!("[device-watcher] started, poll_interval_ms={POLL_INTERVAL_MS}");
            while !stop_thread.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                if stop_thread.load(Ordering::SeqCst) {
                    break;
                }

                let devices = match recorder::list_input_devices() {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("[device-watcher] list_input_devices failed: {e}");
                        continue;
                    }
                };

                let now = snapshot(&devices);
                if now != last {
                    info!(
                        "[device-watcher] change: count {} -> {}, default {:?} -> {:?}",
                        last.0.len(), now.0.len(), last.1, now.1
                    );
                    last = now;
                    on_change(devices);
                }
            }
            info!("[device-watcher] stopped");
        })
        .expect("failed to spawn device-watcher thread");

    WatcherHandle { stop }
}
