//! Custom model YAML plugin support.
//!
//! Scans `~/.voiceink/custom_models/*.yaml`, parses specs, and converts them
//! into `ModelInfo` rows with `provider = ModelProvider::Custom`. The Python
//! daemon does the actual `import` + invocation; this module is purely the
//! Rust-side declarative bridge.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::transcriber::{ModelInfo, ModelProvider};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct LoadSpec {
    pub function: String,
    #[serde(default)]
    pub kwargs: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DownloadSpec {
    #[serde(rename_all(serialize = "camelCase"))]
    HfRepos {
        hf_repos: Vec<String>,
        #[serde(default)]
        paths: HashMap<String, String>,
    },
    #[serde(rename_all(serialize = "camelCase"))]
    Function {
        function: String,
        #[serde(default)]
        kwargs: HashMap<String, serde_yaml::Value>,
        returns: DownloadReturnKind,
        #[serde(default)]
        path_names: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadReturnKind {
    Tuple,
    Dict,
    Path,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelSpec {
    pub source_path: PathBuf,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub size_mb: u32,
    pub languages: HashMap<String, String>,
    pub speed: u8,
    pub accuracy: u8,
    pub recommended: bool,
    pub python_module: String,
    pub pip_packages: Vec<String>,
    pub download: Option<DownloadSpec>,
    pub load: LoadSpec,
    pub transcribe_method: String,
    pub language_param: String,
}

#[derive(Debug, Deserialize)]
struct RawSpec {
    schema_version: u32,
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    size_mb: u32,
    languages: HashMap<String, String>,
    speed: u8,
    accuracy: u8,
    #[serde(default)]
    recommended: bool,
    python_module: String,
    #[serde(default)]
    pip_packages: Option<Vec<String>>,
    #[serde(default)]
    download: Option<DownloadSpec>,
    load: LoadSpec,
    #[serde(default)]
    transcribe_method: Option<String>,
    #[serde(default)]
    language_param: Option<String>,
}

pub fn parse_spec_from_str(yaml: &str, source_path: PathBuf) -> Result<CustomModelSpec, AppError> {
    let raw: RawSpec = serde_yaml::from_str(yaml).map_err(|e| {
        AppError::InvalidInput(format!(
            "YAML parse error in {}: {}",
            source_path.display(),
            e
        ))
    })?;

    let pip_packages = raw
        .pip_packages
        .unwrap_or_else(|| vec![raw.python_module.clone()]);

    Ok(CustomModelSpec {
        source_path,
        schema_version: raw.schema_version,
        id: raw.id,
        name: raw.name,
        description: raw.description,
        size_mb: raw.size_mb,
        languages: raw.languages,
        speed: raw.speed,
        accuracy: raw.accuracy,
        recommended: raw.recommended,
        python_module: raw.python_module,
        pip_packages,
        download: raw.download,
        load: raw.load,
        transcribe_method: raw
            .transcribe_method
            .unwrap_or_else(|| "transcribe".to_string()),
        language_param: raw.language_param.unwrap_or_else(|| "language".to_string()),
    })
}

pub fn parse_spec_from_file(path: &Path) -> Result<CustomModelSpec, AppError> {
    let yaml = std::fs::read_to_string(path)
        .map_err(|e| AppError::Io(format!("read {}: {}", path.display(), e)))?;
    parse_spec_from_str(&yaml, path.to_path_buf())
}

const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[1];

pub fn validate_spec(spec: &CustomModelSpec, existing_ids: &HashSet<String>) -> Result<(), AppError> {
    if !SUPPORTED_SCHEMA_VERSIONS.contains(&spec.schema_version) {
        return Err(AppError::InvalidInput(format!(
            "unsupported schema_version {} (supported: {:?})",
            spec.schema_version, SUPPORTED_SCHEMA_VERSIONS
        )));
    }
    if existing_ids.contains(&spec.id) {
        return Err(AppError::InvalidInput(format!(
            "id '{}' collides with an existing built-in model id",
            spec.id
        )));
    }
    if spec.languages.is_empty() {
        return Err(AppError::InvalidInput("languages must not be empty".into()));
    }
    if !(1..=10).contains(&spec.speed) {
        return Err(AppError::InvalidInput(format!("speed must be 1-10, got {}", spec.speed)));
    }
    if !(1..=10).contains(&spec.accuracy) {
        return Err(AppError::InvalidInput(format!("accuracy must be 1-10, got {}", spec.accuracy)));
    }
    if spec.id.trim().is_empty() {
        return Err(AppError::InvalidInput("id must not be empty".into()));
    }
    if spec.name.trim().is_empty() {
        return Err(AppError::InvalidInput("name must not be empty".into()));
    }
    if spec.python_module.trim().is_empty() {
        return Err(AppError::InvalidInput("python_module must not be empty".into()));
    }
    Ok(())
}

#[derive(Debug)]
pub enum ScanResult {
    Ok(CustomModelSpec),
    Err { path: PathBuf, error: String },
}

/// Convert a parsed `CustomModelSpec` into a `ModelInfo` row for the
/// model registry. Mirrors the `langs_to_vec` convention used by built-in
/// providers (>1 language → `["multi"]`).
pub fn spec_to_model_info(spec: &CustomModelSpec) -> ModelInfo {
    let languages = if spec.languages.len() > 1 {
        vec!["multi".to_string()]
    } else {
        spec.languages.keys().cloned().collect()
    };

    ModelInfo {
        id: spec.id.clone(),
        name: spec.name.clone(),
        size_mb: spec.size_mb,
        supported_languages: spec.languages.clone(),
        languages,
        download_url: String::new(),
        is_downloaded: false,
        provider: ModelProvider::Custom,
        // Repurpose `model_repo` to point at the YAML spec file path, so
        // downstream consumers can locate the source plugin.
        model_repo: Some(spec.source_path.to_string_lossy().into_owned()),
        description: spec.description.clone(),
        speed: spec.speed,
        accuracy: spec.accuracy,
        is_recommended: spec.recommended,
    }
}

/// Build the daemon `load` command JSON for a given model_repo.
///
/// Custom-model YAML paths (ending in `.yaml`/`.yml`, case-insensitive)
/// get `provider: "custom"` plus `voiceink_models_dir` and `custom_models_dir`
/// injected so the daemon routes to `load_custom_model`. All other model_repos
/// get the standard `{action: "load", model}` shape.
pub fn build_load_command(model_repo: &str) -> serde_json::Value {
    let mut cmd = serde_json::json!({
        "action": "load",
        "model": model_repo,
    });
    let lower = model_repo.to_ascii_lowercase();
    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        let home = dirs::home_dir().unwrap_or_default();
        cmd["provider"] = serde_json::json!("custom");
        cmd["voiceink_models_dir"] = serde_json::json!(
            home.join(".voiceink/models").to_string_lossy()
        );
        cmd["custom_models_dir"] = serde_json::json!(
            home.join(".voiceink/custom_models").to_string_lossy()
        );
    }
    cmd
}

pub fn scan_custom_models(dir: &Path) -> Vec<ScanResult> {
    if !dir.exists() {
        return Vec::new();
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("[custom_models] read_dir {} failed: {}", dir.display(), e);
            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("[custom_models] entry read error in {}: {}", dir.display(), e);
                continue;
            }
        };
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        match parse_spec_from_file(&path) {
            Ok(spec) => out.push(ScanResult::Ok(spec)),
            Err(e) => out.push(ScanResult::Err {
                path: path.clone(),
                error: e.to_string(),
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::build_load_command;

    #[test]
    fn build_load_cmd_adds_provider_for_yaml_path() {
        let cmd = build_load_command("/Users/x/.voiceink/custom_models/mimo.yaml");
        assert_eq!(cmd["action"], "load");
        assert_eq!(cmd["model"], "/Users/x/.voiceink/custom_models/mimo.yaml");
        assert_eq!(cmd["provider"], "custom");
        assert!(cmd["voiceink_models_dir"].is_string());
        assert!(cmd["custom_models_dir"].is_string());
    }

    #[test]
    fn build_load_cmd_adds_provider_for_yml_path() {
        let cmd = build_load_command("/tmp/specs/foo.yml");
        assert_eq!(cmd["provider"], "custom");
        assert!(cmd["voiceink_models_dir"].is_string());
    }

    #[test]
    fn build_load_cmd_handles_uppercase_extension() {
        let cmd = build_load_command("/tmp/Spec.YAML");
        assert_eq!(cmd["provider"], "custom");
    }

    #[test]
    fn build_load_cmd_no_provider_for_normal_model() {
        let cmd = build_load_command("ggml-tiny");
        assert_eq!(cmd["action"], "load");
        assert_eq!(cmd["model"], "ggml-tiny");
        assert!(cmd.get("provider").is_none());
        assert!(cmd.get("voiceink_models_dir").is_none());
        assert!(cmd.get("custom_models_dir").is_none());
    }

    #[test]
    fn build_load_cmd_no_provider_for_hf_repo() {
        let cmd = build_load_command("mlx-community/Fun-ASR-MLT-Nano-2512-8bit");
        assert!(cmd.get("provider").is_none());
    }
}
