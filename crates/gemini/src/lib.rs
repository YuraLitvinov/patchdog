use async_trait::async_trait;
use gemini_rust::Gemini;
use std::error::Error as Err;
#[async_trait]
pub trait ReqRes {
    async fn req_res(file_content: String) -> Result<String, Box<dyn Err>>;
}
pub struct GoogleGemini; //Req Res = Request Response
#[allow(async_fn_in_trait)]
impl GoogleGemini {
    pub async fn req_res(file_content: String) -> Result<String, Box<dyn Err>> {
        let api_key = std::env::var("API_KEY_GEMINI")?;
        let client = Gemini::new(&api_key);
        let args = std::env::var("INPUT_FOR_MODEL")?;
        let res = client
            .generate_content()
            .with_system_prompt(args)
            .with_user_message(file_content)
            .execute()
            .await?;
        Ok(res.text())
    }
}
