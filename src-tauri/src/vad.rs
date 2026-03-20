use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadResult {
    pub is_speech: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
}

pub struct ChunkManager {
    current_speech: Vec<f32>,
    ready_chunks: Vec<AudioChunk>,
    in_speech: bool,
    silence_timeout_ms: Option<u64>,
    silence_start_ms: Option<u64>,
}

impl ChunkManager {
    pub fn new() -> Self {
        Self {
            current_speech: Vec::new(),
            ready_chunks: Vec::new(),
            in_speech: false,
            silence_timeout_ms: None,
            silence_start_ms: None,
        }
    }

    pub fn with_silence_timeout_ms(timeout_ms: u64) -> Self {
        Self {
            silence_timeout_ms: Some(timeout_ms),
            ..Self::new()
        }
    }

    pub fn feed_samples(&mut self, samples: &[f32], is_speech: bool) {
        if is_speech {
            if !self.in_speech {
                info!("[vad] [speech_start]");
            }
            self.in_speech = true;
            self.silence_start_ms = None;
            self.current_speech.extend_from_slice(samples);
        } else if self.in_speech {
            // Log speech_end only on the first silence frame (transition)
            if self.silence_start_ms.is_none() && self.silence_timeout_ms.is_none() {
                info!("[vad] [speech_end] pending_samples={}", self.current_speech.len());
            }
            if self.silence_timeout_ms.is_none() {
                // No timeout -- immediate split
                self.flush_current_chunk();
            }
            // With timeout, wait for feed_samples_with_timestamp
        }
    }

    pub fn feed_samples_with_timestamp(
        &mut self,
        samples: &[f32],
        is_speech: bool,
        elapsed_ms: u64,
    ) {
        if is_speech {
            if !self.in_speech {
                info!("[vad] [speech_start]");
            }
            self.in_speech = true;
            self.silence_start_ms = None;
            self.current_speech.extend_from_slice(samples);
        } else if self.in_speech {
            if self.silence_start_ms.is_none() {
                self.silence_start_ms = Some(elapsed_ms);
            }
            if let Some(timeout) = self.silence_timeout_ms {
                if elapsed_ms >= timeout {
                    info!("[vad] [silence_timeout] elapsed_ms={} timeout_ms={}", elapsed_ms, timeout);
                    self.flush_current_chunk();
                }
            }
        }
    }

    fn flush_current_chunk(&mut self) {
        let chunk_len = self.current_speech.len();
        if !self.current_speech.is_empty() {
            self.ready_chunks.push(AudioChunk {
                samples: std::mem::take(&mut self.current_speech),
            });
        }
        self.in_speech = false;
        self.silence_start_ms = None;
        info!("[vad] [flush] samples={} ready_chunks={}", chunk_len, self.ready_chunks.len());
    }

    pub fn pending_chunks(&self) -> usize {
        self.ready_chunks.len()
    }

    pub fn take_chunk(&mut self) -> Option<AudioChunk> {
        if self.ready_chunks.is_empty() {
            None
        } else {
            Some(self.ready_chunks.remove(0))
        }
    }

    pub fn merge_all_chunks(&mut self) -> Vec<f32> {
        // Also flush any remaining speech
        if !self.current_speech.is_empty() {
            self.flush_current_chunk();
        }
        let chunks_count = self.ready_chunks.len();
        let mut merged = Vec::new();
        for chunk in self.ready_chunks.drain(..) {
            merged.extend(chunk.samples);
        }
        info!("[vad] [merge] total_samples={} chunks={}", merged.len(), chunks_count);
        merged
    }
}
