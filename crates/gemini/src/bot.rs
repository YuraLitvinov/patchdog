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

/// Asynchronously sends a code snippet (`file_content`) to the Google Gemini Large Language Model for processing. It retrieves the Gemini API key from the `API_KEY_GEMINI` environment variable and constructs the prompt and model settings from the application's configuration.
///
/// # Arguments
/// * `file_content` - A string slice containing the code or text to be sent to the Gemini LLM.
///
/// # Returns
/// A `Result<String, ErrorHandling>` containing the text response from the Gemini LLM on success, or an `ErrorHandling` if the API key is missing, configuration cannot be loaded, or the API call fails.
impl RequestResponseConstruction for AiRequest {
/// Determines which Large Language Model (LLM) to use for a given `file_content` based on the `llm_model` specified in the application's configuration. It dynamically dispatches the request to either the OpenAI or Google Gemini LLM API.
///
/// # Arguments
/// * `file_content` - A string slice containing the content to be processed by the selected LLM.
///
/// # Returns
/// A `Result<String, ErrorHandling>` containing the response from the chosen LLM on success, or an `ErrorHandling` if the configuration cannot be read, the specified model is unsupported, or the LLM call fails.
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

/// Asynchronously sends a code snippet (`file_content`) to the OpenAI Large Language Model for processing. It retrieves the OpenAI API key from the `API_KEY_OPENAI` environment variable and configures the chat model and initial prompt based on the application's settings.
///
/// # Arguments
/// * `file_content` - A string slice containing the code or text to be sent as a user message to the OpenAI LLM.
///
/// # Returns
/// A `Result<String, ErrorHandling>` containing the text response from the OpenAI LLM on success, or an `ErrorHandling` if the API key is missing, configuration cannot be loaded, or the API call fails.
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
