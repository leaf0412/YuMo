use yumo_lib::vad;

#[test]
fn test_chunk_manager_accumulates_speech() {
    let mut mgr = vad::ChunkManager::new();
    mgr.feed_samples(&[0.1f32; 4800], true); // speech
    mgr.feed_samples(&[0.1f32; 4800], true); // speech continues
    assert_eq!(mgr.pending_chunks(), 0); // no speech end yet
}

#[test]
fn test_chunk_manager_splits_on_silence() {
    let mut mgr = vad::ChunkManager::new();
    mgr.feed_samples(&[0.1f32; 4800], true); // speech
    mgr.feed_samples(&[0.1f32; 4800], true); // speech continues
    mgr.feed_samples(&[0.0f32; 4800], false); // silence -> triggers chunk
    assert_eq!(mgr.pending_chunks(), 1);

    let chunk = mgr.take_chunk().unwrap();
    assert_eq!(chunk.samples.len(), 9600); // only speech frames
}

#[test]
fn test_chunk_manager_multiple_chunks() {
    let mut mgr = vad::ChunkManager::new();

    // Chunk 1
    mgr.feed_samples(&[0.1f32; 4800], true);
    mgr.feed_samples(&[0.0f32; 4800], false);
    // Chunk 2
    mgr.feed_samples(&[0.2f32; 4800], true);
    mgr.feed_samples(&[0.0f32; 4800], false);

    assert_eq!(mgr.pending_chunks(), 2);
}

#[test]
fn test_chunk_manager_merge_all() {
    let mut mgr = vad::ChunkManager::new();

    mgr.feed_samples(&[0.1f32; 4800], true);
    mgr.feed_samples(&[0.0f32; 4800], false);
    mgr.feed_samples(&[0.2f32; 4800], true);
    mgr.feed_samples(&[0.0f32; 4800], false);

    let merged = mgr.merge_all_chunks();
    assert_eq!(merged.len(), 9600); // two speech segments of 4800 each
}

#[test]
fn test_chunk_manager_take_returns_none_when_empty() {
    let mut mgr = vad::ChunkManager::new();
    assert!(mgr.take_chunk().is_none());
}

#[test]
fn test_chunk_manager_silence_only_no_chunks() {
    let mut mgr = vad::ChunkManager::new();
    mgr.feed_samples(&[0.0f32; 4800], false);
    mgr.feed_samples(&[0.0f32; 4800], false);
    assert_eq!(mgr.pending_chunks(), 0);
}

#[test]
fn test_chunk_manager_with_silence_timeout() {
    let mut mgr = vad::ChunkManager::with_silence_timeout_ms(500);
    // Speech then short silence (< timeout)
    mgr.feed_samples(&[0.1f32; 4800], true);
    mgr.feed_samples_with_timestamp(&[0.0f32; 4800], false, 100); // 100ms silence
    assert_eq!(mgr.pending_chunks(), 0); // not enough silence

    // More silence past timeout
    mgr.feed_samples_with_timestamp(&[0.0f32; 4800], false, 600); // 600ms total
    assert_eq!(mgr.pending_chunks(), 1);
}

#[test]
fn test_vad_result_struct() {
    let result = vad::VadResult {
        is_speech: true,
        confidence: 0.95,
    };
    assert!(result.is_speech);
    assert!(result.confidence > 0.9);
}
