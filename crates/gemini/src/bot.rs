use ai_interactions::return_prompt;
use gemini_rust::Gemini;
use openai_rust::Client;
use rust_parsing::ErrorHandling;
use std::env::var;

#[allow(async_fn_in_trait)]
pub trait RequestResponseConstruction {
    async fn switch_llm(file_content: &str) -> Result<String, ErrorHandling>;
    async fn call_llm_gemini(file_content: &str) -> Result<String, ErrorHandling>;
    async fn call_llm_openai(file_content: &str) -> Result<String, ErrorHandling>;
}

pub struct AiRequest;

impl RequestResponseConstruction for AiRequest {
    async fn switch_llm(file_content: &str) -> Result<String, ErrorHandling> {
        let yaml = return_prompt()?;
        let model = yaml.patchdog_settings.llm_model.as_str();
        //println!("{:#?}", yaml);
        match model {
            "openai" => AiRequest::call_llm_openai(file_content).await,
            "google" => AiRequest::call_llm_gemini(file_content).await,
            _ => Ok(format!("Specified model {} is not supported", model)),
        }
    }

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

    async fn call_llm_openai(file_content: &str) -> Result<String, ErrorHandling> {
        let api_key = var("API_KEY_OPENAI")?;
        let client = Client::new(&api_key);
        let args = openai_rust::chat::ChatArguments::new(
            return_prompt()?.llm_settings.openai_model,
            vec![
                openai_rust::chat::Message {
                    role: "user".to_owned(),
                    content: return_prompt()?.prompt.to_string(),
                },
                openai_rust::chat::Message {
                    role: "user".to_owned(),
                    content: file_content.to_owned(),
                },
            ],
        );
        let res = client.create_chat(args).await?;
        Ok(res.to_string())
    }
}
