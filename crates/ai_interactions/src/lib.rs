pub mod parse_json;
use rust_parsing::ErrorHandling;
use std::fs;
use yaml_rust2::{Yaml, YamlLoader};

pub struct YamlRead {
    pub prompt: String,
    pub model: String,
    pub tokens: usize,
    pub requests: usize,
}

/// Reads configuration from `config.yaml` into a `YamlRead` struct.
///
/// This function opens and reads `config.yaml`, parsing the `prompt`, `GEMINI_MODEL`,
/// `TOKENS_PER_MIN`, and `REQUESTS_PER_MIN` fields. Defaults are applied if fields are missing or
/// malformed.
///
/// # Returns
///
/// - `Ok(YamlRead)`: A `YamlRead` struct containing the parsed configuration values.
/// - `Err(ErrorHandling)`: If the file cannot be read, parsed, or other processing issues occur.
/// Reads configuration from `config.yaml` and parses specific fields into a `YamlRead` struct.
///
/// This function attempts to open and read `config.yaml`. It then parses the YAML content
/// to extract the following fields:
/// - `prompt`: A string, defaulting to an empty string if not found.
/// - `GEMINI_MODEL`: A string, defaulting to an empty string if not found.
/// - `TOKENS_PER_MIN`: An integer converted to `usize`, defaulting to 0 if not found or not an integer.
/// - `REQUESTS_PER_MIN`: An integer converted to `usize`, defaulting to 0 if not found or not an integer.
///
/// # Returns
///
/// - `Ok(YamlRead)`: A `Result` containing a `YamlRead` struct with the extracted configuration values.
/// - `Err(ErrorHandling)`: An error if the file cannot be read, parsed, or if other issues occur during processing.
pub fn return_prompt() -> Result<YamlRead, ErrorHandling> {
    let config = fs::read_to_string("config.yaml")?;
    let docs = YamlLoader::load_from_str(&config)?;
    let doc = &docs[0];
    if let Yaml::Hash(h) = doc {
        let prompt = h
            .get(&Yaml::String("prompt".into()))
            .and_then(|v| match v {
                Yaml::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default(); // "" if missing

        let model = h
            .get(&Yaml::String("GEMINI_MODEL".into()))
            .and_then(|v| match v {
                Yaml::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let tokens = h
            .get(&Yaml::String("TOKENS_PER_MIN".into()))
            .and_then(|v| match v {
                Yaml::Integer(i) => Some(*i as usize),
                _ => None,
            })
            .unwrap_or(0);

        let requests = h
            .get(&Yaml::String("REQUESTS_PER_MIN".into()))
            .and_then(|v| match v {
                Yaml::Integer(i) => Some(*i as usize),
                _ => None,
            })
            .unwrap_or(0);

        Ok(YamlRead {
            prompt,
            model,
            tokens,
            requests,
        })
    } else {
        Ok(YamlRead {
            prompt: "".to_string(),
            model: "".to_string(),
            tokens: 0,
            requests: 0,
        })
    }
}
