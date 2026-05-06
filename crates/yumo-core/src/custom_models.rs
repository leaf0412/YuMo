//! Custom model YAML plugin support.
//!
//! Scans `~/.voiceink/custom_models/*.yaml`, parses specs, and converts them
//! into `ModelInfo` rows with `provider = ModelProvider::Custom`. The Python
//! daemon does the actual `import` + invocation; this module is purely the
//! Rust-side declarative bridge.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct CustomModelSpec {
    pub source_path: PathBuf,
}
