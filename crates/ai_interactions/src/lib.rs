pub mod parse_json;
use std::fs;
use rust_parsing::ErrorHandling;
use yaml_rust2::{Yaml, YamlLoader};
/// Returns a static string containing a prompt for the Google Gemini API.
pub fn return_prompt() -> Result<String, ErrorHandling> {
    let config = fs::read_to_string("config.yaml")?;
    let docs = YamlLoader::load_from_str(&config)?;
    let doc = &docs[0];
    if let Yaml::Hash(h) = doc {
        if let Some(Yaml::String(prompt)) = h.get(&Yaml::String("prompt".to_string())) {
            Ok(prompt.to_string())
        }
        else {
            Ok("".into())
        }
    }
    else {
         Ok("".into())
    }
    
}
