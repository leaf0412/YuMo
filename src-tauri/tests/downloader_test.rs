use yumo_lib::downloader;

#[tokio::test]
async fn test_download_small_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dest = tmp.path().join("test_download.txt");

    // Use a very small known file for fast testing
    let url = "https://raw.githubusercontent.com/ggerganov/whisper.cpp/master/LICENSE";

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(100);

    let dest_clone = dest.clone();
    let handle = tokio::spawn(async move {
        downloader::download_file(url, &dest_clone, Some(progress_tx))
            .await
            .unwrap();
    });

    let mut got_progress = false;
    while let Some(progress) = progress_rx.recv().await {
        got_progress = true;
        assert!(progress >= 0.0 && progress <= 1.0);
    }

    handle.await.unwrap();
    assert!(got_progress);
    assert!(dest.exists());
    assert!(std::fs::metadata(&dest).unwrap().len() > 100);
}

#[tokio::test]
async fn test_download_without_progress() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dest = tmp.path().join("test_no_progress.txt");

    let url = "https://raw.githubusercontent.com/ggerganov/whisper.cpp/master/LICENSE";
    downloader::download_file(url, &dest, None).await.unwrap();

    assert!(dest.exists());
}

#[tokio::test]
async fn test_download_invalid_url_fails() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dest = tmp.path().join("should_not_exist.txt");

    let result =
        downloader::download_file("https://nonexistent.example.com/file.bin", &dest, None).await;

    assert!(result.is_err());
    assert!(!dest.exists());
}
