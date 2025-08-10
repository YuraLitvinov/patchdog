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
