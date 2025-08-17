pub mod parse_json;
use rust_parsing::{error::InvalidIoOperationsSnafu, ErrorHandling};
use snafu::ResultExt;
use std::fs;
use yaml_rust2::{Yaml, YamlLoader};
use std::path::Path;
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
    pub excluded_functions: Vec<String>
}

#[derive(Debug)]
pub struct YamlRead {
    pub prompt: String,
    pub llm_settings: LLMSettings,
    pub patchdog_settings: PathdogSettings
}

/// Reads and parses the `config.yaml` file to extract various application settings. 
/// This function is responsible for loading the YAML configuration, specifically retrieving a prompt string, Large Language Model (LLM) settings such as model names and rate limits (tokens/requests per minute), and Patchdog-specific exclusions for files and functions. 
/// It returns a `Result` containing a `YamlRead` struct populated with these parsed settings, applying default values if any configuration items are missing or malformed, or an `ErrorHandling` enum if an I/O or parsing error occurs.
/// Reads and parses the `config.yaml` file to extract application settings.
/// This function attempts to load the YAML configuration, specifically looking for a prompt string, LLM model settings (like OpenAI and Gemini models, token/request limits), and Patchdog-specific settings (such as excluded files and functions).
/// It returns a `Result` containing a `YamlRead` struct populated with the parsed configuration or an `ErrorHandling` enum if file operations or YAML parsing fails. Default values are provided for settings that are not found or are malformed in the configuration file.
pub fn return_prompt() -> Result<YamlRead, ErrorHandling> {
    let path = Path::new("config.yaml").to_path_buf();
    let config = fs::read_to_string(&path).context(InvalidIoOperationsSnafu { path })?;
    let docs = YamlLoader::load_from_str(&config)?;
    let doc = &docs[0];
    if let Yaml::Hash(patchdog) = doc {
        let hashes = &patchdog.into_iter().map(|hash|hash.1.to_owned() ).collect::<Vec<Yaml>>()[0]; 
        if let Yaml::Hash(h) = hashes {
        let prompt = h
            .get(&Yaml::String("prompt".into()))
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        let llm_settings = h
            .get(&Yaml::String("LLM_settings".into())).map(|v| v.as_hash().unwrap()).unwrap();
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
            .and_then(|v| v.as_i64().map(|i| i as usize)).unwrap();
        let requests = llm_settings
            .get(&Yaml::String("REQUESTS_PER_MIN".into()))
            .and_then(|v| v.as_i64().map(|i| i as usize))
            .unwrap();
        let patchdog_settings = h
            .get(&Yaml::String("Patchdog_settings".into())).map(|v| v.as_hash().unwrap()).unwrap();
        let excluded_files = patchdog_settings
            .get(&Yaml::String("excluded_files".into()))
            .and_then(|v| v.as_vec())
            .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let excluded_functions = patchdog_settings
            .get(&Yaml::String("excluded_functions".into()))
            .and_then(|v| v.as_vec())
            .map(|arr| arr.iter().filter_map(|item| item.as_str().map(String::from)).collect())
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
            },
        })
        }
        else {
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
            },
        })
        }


    } else {
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
            },
        })
    }
}
