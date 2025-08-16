use ai_interactions::YamlRead;
use rust_parsing::ErrorHandling;
use std::env::var;
use ai_interactions::return_prompt;
use gemini_rust::Gemini;

#[allow(async_fn_in_trait)]
pub trait RequestResponseConstruction {
    async fn call_llm_gemini(file_content: &str, arguments: YamlRead) -> Result<String, ErrorHandling>;
}

pub struct AiRequest;

impl RequestResponseConstruction for AiRequest {
    async fn call_llm_gemini(file_content: &str, arguments: YamlRead) -> Result<String, ErrorHandling> {
        let api_key = var("API_KEY_GEMINI")?;
        let model = return_prompt()?.llm_settings.gemini_model;
        let client = Gemini::with_model(api_key, model)
            .generate_content()
            .with_system_prompt(arguments.prompt)
            .with_user_message(file_content)
            .execute()
            .await?;
        Ok(client.text())
    }

}