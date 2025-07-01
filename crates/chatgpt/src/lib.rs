use std::error::Error as Err;
pub struct OpenAI;
impl OpenAI {
   pub async fn req_res(file_content: String) -> Result<String, Box<dyn Err>> {
        let api_key = std::env::var("API_KEY_OPENAI")?;
        let client = openai_rust::Client::new(&std::env::var(api_key).unwrap());
        let args = openai_rust::chat::ChatArguments::new(&std::env::var("OPENAI_MODEL").expect("Unsupported OpenAI model"), vec![
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: "Generate docstring for this rust script (rustdoc type); document every mentioned object in the exhaustive list".to_owned(),

            },
            openai_rust::chat::Message {
                role: "user".to_owned(),
                content: file_content.to_owned(),

            },
        ]);
        let res = client.create_chat(args).await.unwrap();
        Ok(res.to_string())
    }
}