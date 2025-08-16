use rust_parsing::ErrorHandling;
use std::env::var;
use ai_interactions::return_prompt;
use gemini_rust::Gemini;

#[allow(async_fn_in_trait)]
pub trait RequestResponseConstruction {
    async fn call_llm_gemini(file_content: &str) -> Result<String, ErrorHandling>;
    async fn call_llm_openai(file_content: String) -> Result<String, ErrorHandling>;
}

pub struct AiRequest;
/*Plans:
    1. Switch function to choose between different models
    2. More LLMs that current gemini
*/ 
impl RequestResponseConstruction for AiRequest {
    async fn call_llm_gemini(file_content: &str) -> Result<String, ErrorHandling> {
        let api_key = var("API_KEY_GEMINI")?;
        let model = return_prompt()?.llm_settings.gemini_model;
        let client = Gemini::with_model(api_key, model)
            .generate_content()
            .with_system_prompt(return_prompt()?.prompt)
            .with_user_message(file_content)
            .execute()
            .await?;
        Ok(client.text())
    }
    async fn call_llm_openai(file_content: String) -> Result<String, ErrorHandling> {
        let api_key = std::env::var("API_KEY_OPENAI")?;
        let client = openai_rust::Client::new(&api_key);
        let args = openai_rust::chat::ChatArguments::new(std::env::var("OPENAI_MODEL").expect("Unsupported OpenAI model"), vec![
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: return_prompt()?.prompt.to_string(),

            },
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: file_content.to_owned(),

            },
        ]);
        let res = client.create_chat(args).await?;
        Ok(res.to_string())
    }
}