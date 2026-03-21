use crate::error::AppError;
use crate::platform::types::AudioData;
use log::{error, info};
use std::path::{Path, PathBuf};

/// Save a recording to the given directory as a timestamped WAV file.
/// Returns the full path of the saved file.
pub fn save_recording(data: &AudioData, dir: &Path) -> Result<PathBuf, AppError> {
    info!(
        "[audio_io] save_recording dir={:?} samples={} sample_rate={}",
        dir,
        data.pcm_samples.len(),
        data.sample_rate
    );
    std::fs::create_dir_all(dir)?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("recording_{}.wav", timestamp);
    let path = dir.join(filename);
    save_wav(data, &path)?;
    info!("[audio_io] save_recording saved to {:?}", path);
    Ok(path)
}

pub fn save_wav(data: &AudioData, path: &Path) -> Result<(), AppError> {
    info!(
        "[audio_io] save_wav path={:?} samples={}",
        path,
        data.pcm_samples.len()
    );
    let spec = hound::WavSpec {
        channels: data.channels,
        sample_rate: data.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| {
        error!("[audio_io] failed to create WAV writer at {:?}: {}", path, e);
        AppError::Io(e.to_string())
    })?;
    for &sample in &data.pcm_samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(s)
            .map_err(|e| AppError::Io(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| AppError::Io(e.to_string()))?;
    Ok(())
}

/// Read a WAV file and return it as a base64 data URI for frontend playback.
pub fn read_recording_as_data_uri(path: &Path) -> Result<String, AppError> {
    info!("[audio_io] read_recording_as_data_uri path={:?}", path);
    if !path.exists() {
        error!("[audio_io] recording not found: {}", path.display());
        return Err(AppError::NotFound(format!(
            "Recording not found: {}",
            path.display()
        )));
    }
    let data = std::fs::read(path)?;
    info!(
        "[audio_io] read_recording_as_data_uri size={} bytes",
        data.len()
    );
    let b64 = base64_encode(&data);
    Ok(format!("data:audio/wav;base64,{}", b64))
}

/// Minimal base64 encoder (no external dependency).
pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}
