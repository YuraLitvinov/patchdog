pub mod parse_json;
use rust_parsing::ErrorHandling;
use std::fs;
use yaml_rust2::{Yaml, YamlLoader};
/// Reads the 'prompt' value from a 'config.yaml' file and returns it as a `String`.
/// If the file cannot be read, the YAML cannot be loaded, or the 'prompt' key is not found, an empty `String` is returned or an `ErrorHandling` is propagated.
///
/// # Returns
///
/// A `Result` containing the prompt `String` or an `ErrorHandling` if an error occurs during file operations or YAML parsing.
/// Returns a static string containing a prompt for the Google Gemini API.
pub fn return_prompt() -> Result<String, ErrorHandling> {
    let config = fs::read_to_string("config.yaml")?;
    let docs = YamlLoader::load_from_str(&config)?;
    let doc = &docs[0];
    if let Yaml::Hash(h) = doc {
        if let Some(Yaml::String(prompt)) = h.get(&Yaml::String("prompt".to_string())) {
            Ok(prompt.to_string())
        } else {
            Ok("".into())
        }
    } else {
        Ok("".into())
    }
}
