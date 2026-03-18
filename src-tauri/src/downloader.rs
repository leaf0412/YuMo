use crate::error::AppError;
use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

/// Download a file from URL to dest path, optionally reporting progress (0.0-1.0).
pub async fn download_file(
    url: &str,
    dest: &Path,
    progress: Option<mpsc::Sender<f32>>,
) -> Result<(), AppError> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!("HTTP {}", response.status())));
    }

    let total_size = response.content_length();
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Network(e.to_string()))?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let (Some(total), Some(tx)) = (total_size, &progress) {
            let pct = downloaded as f32 / total as f32;
            let _ = tx.send(pct.min(1.0)).await;
        }
    }

    file.flush().await?;

    Ok(())
}
