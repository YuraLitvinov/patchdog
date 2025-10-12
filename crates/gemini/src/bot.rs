use ai_interactions::return_prompt;
use gemini_rust::Gemini;
use openai_rust::Client;
use rust_parsing::ErrorHandling;
use snafu::ResultExt;
use rust_parsing::error::StdVarSnafu;

#[allow(async_fn_in_trait)]
pub trait RequestResponseConstruction {
    async fn switch_llm(file_content: &str) -> Result<String, ErrorHandling>;
    async fn call_llm_gemini(file_content: &str) -> Result<String, ErrorHandling>;
    async fn call_llm_openai(file_content: &str) -> Result<String, ErrorHandling>;
}

pub struct AiRequest;

impl RequestResponseConstruction for AiRequest {
    /// Dispatches the provided `file_content` to the appropriate Large Language Model (LLM) based on the configured `llm_model` in the application's settings. It dynamically selects between OpenAI and Google's Gemini models for processing. If the specified model is not supported, an informative error message is returned.
    ///
    /// # Arguments
    /// * `file_content` - A string slice containing the content to be processed by the LLM.
    ///
    /// # Returns
    /// A `Result<String, ErrorHandling>` containing the LLM's response as a string on success, or an `ErrorHandling` if configuration fails or an unsupported model is specified.
    async fn switch_llm(file_content: &str) -> Result<String, ErrorHandling> {
        let yaml = return_prompt()?;
        let model = yaml.patchdog_settings.llm_model.as_str();
        match model {
            "openai" => AiRequest::call_llm_openai(file_content).await,
            "google" => AiRequest::call_llm_gemini(file_content).await,
            _ => Ok(format!("Specified model {} is not supported", model)),
        }
    }

    /// Asynchronously sends a code snippet (`file_content`) to the Google Gemini Large Language Model for processing. It retrieves the Gemini API key from the `API_KEY_GEMINI` environment variable and configures the chat model and initial prompt based on the application's settings.
    ///
    /// # Arguments
    /// * `file_content` - A string slice containing the code or text to be sent as a user message to the Gemini LLM.
    ///
    /// # Returns
    /// A `Result<String, ErrorHandling>` containing the text response from the Gemini LLM on success, or an `ErrorHandling` if the API key is missing, configuration cannot be loaded, or the API call fails.
    async fn call_llm_gemini(file_content: &str) -> Result<String, ErrorHandling> { 
        let varname = "API_KEY_GEMINI";
        let api_key = std::env::var(varname)
            .context(StdVarSnafu {  varname: varname.to_string() })?;

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
        let varname = "API_KEY_OPENAI";
        let api_key = std::env::var(varname)
            .context(StdVarSnafu {  varname: varname.to_string() })?;

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
