use rust_parsing::{ErrorHandling, error::InvalidIoOperationsSnafu};
use snafu::ResultExt;
use std::fs;
use std::path::Path;
use tracing::{Level, event};
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug)]
pub struct LLMSettings {
    pub openai_model: String,
    pub gemini_model: String,
    pub tokens: usize,
    pub requests: usize,
}

#[derive(Debug)]
pub struct PathdogSettings {
    pub excluded_files: Vec<String>,
    pub excluded_functions: Vec<String>,
    pub llm_model: String,
}

#[derive(Debug)]
pub struct YamlRead {
    pub prompt: String,
    pub llm_settings: LLMSettings,
    pub patchdog_settings: PathdogSettings,
}

/// Reads configuration settings from a YAML file specified by the `CONFIG_PATH` environment variable.
/// It parses the YAML content, extracting `prompt` text, LLM settings (model names, tokens/requests per minute), and Patchdog-specific configurations like `excluded_files` and `excluded_functions`.
/// Returns a `YamlRead` struct containing all parsed settings or an `ErrorHandling` if the file cannot be read, parsed, or required keys are missing, providing default values in case of partial failures.
pub fn return_prompt() -> Result<YamlRead, ErrorHandling> {
    let path = Path::new(&std::env::var("CONFIG_PATH")?).to_path_buf();
    let config =
        fs::read_to_string(&path).context(InvalidIoOperationsSnafu { path: path.clone() })?;
    let docs = YamlLoader::load_from_str(&config)?;
    let doc = &docs[0];
    if let Yaml::Hash(patchdog) = doc {
        let hashes = &patchdog
            .into_iter()
            .map(|hash| hash.1.to_owned())
            .collect::<Vec<Yaml>>()[0];
        if let Yaml::Hash(h) = hashes {
            let prompt = h
                .get(&Yaml::String("prompt".into()))
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let llm_settings = h
                .get(&Yaml::String("LLM_settings".into()))
                .map(|v| v.as_hash().unwrap())
                .unwrap();
            let openai_model = llm_settings
                .get(&Yaml::String("OPENAI_MODEL".into()))
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let gemini_model = llm_settings
                .get(&Yaml::String("GEMINI_MODEL".into()))
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let tokens = llm_settings
                .get(&Yaml::String("TOKENS_PER_MIN".into()))
                .and_then(|v| v.as_i64().map(|i| i as usize))
                .unwrap();
            let requests = llm_settings
                .get(&Yaml::String("REQUESTS_PER_MIN".into()))
                .and_then(|v| v.as_i64().map(|i| i as usize))
                .unwrap();
            let patchdog_settings = h
                .get(&Yaml::String("Patchdog_settings".into()))
                .map(|v| v.as_hash().unwrap())
                .unwrap();
            let excluded_files = patchdog_settings
                .get(&Yaml::String("excluded_files".into()))
                .and_then(|v| v.as_vec())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let excluded_functions = patchdog_settings
                .get(&Yaml::String("excluded_functions".into()))
                .and_then(|v| v.as_vec())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let llm_model = patchdog_settings
                .get(&Yaml::String("llm_model".into()))
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();

            Ok(YamlRead {
                prompt,
                llm_settings: LLMSettings {
                    openai_model,
                    gemini_model,
                    tokens,
                    requests,
                },
                patchdog_settings: PathdogSettings {
                    excluded_files,
                    excluded_functions,
                    llm_model,
                },
            })
        } else {
            event!(
                Level::ERROR,
                "No proper configuration provided inside {}, at patchdog key",
                path.display()
            );
            Ok(YamlRead {
                prompt: "".to_string(),
                llm_settings: LLMSettings {
                    openai_model: "".to_string(),
                    gemini_model: "".to_string(),
                    tokens: 0,
                    requests: 0,
                },
                patchdog_settings: PathdogSettings {
                    excluded_files: vec![],
                    excluded_functions: vec![],
                    llm_model: String::new(),
                },
            })
        }
    } else {
        event!(
            Level::ERROR,
            "Couldn't find patchdog key in {}",
            path.display()
        );
        // Default config if YAML isn't structured properly
        Ok(YamlRead {
            prompt: "".to_string(),
            llm_settings: LLMSettings {
                openai_model: "".to_string(),
                gemini_model: "".to_string(),
                tokens: 0,
                requests: 0,
            },
            patchdog_settings: PathdogSettings {
                excluded_files: vec![],
                excluded_functions: vec![],
                llm_model: String::new(),
            },
        })
    }
}
