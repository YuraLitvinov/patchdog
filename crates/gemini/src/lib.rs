use std::fmt::{self, Display};

use async_trait::async_trait;
use gemini_rust::Gemini;
use rust_parsing::{error::{GeminiRustSnafu, InvalidIoOperationsSnafu, StdVarSnafu}, ErrorHandling};
use snafu::ResultExt;
use serde_json::json;

const TOKENS_PER_MIN: usize = 250_000;
pub const REQUESTS_PER_MIN: usize = 5;
const TOKENS_PER_REQUEST: usize = TOKENS_PER_MIN/REQUESTS_PER_MIN;
#[derive(Debug)]
pub struct SingleRequestData {
    pub function_text: String,
    pub context: String,
    pub comment: String,
    pub filepath: String,
}
impl SingleRequestData {
    pub fn size(&self) -> usize {
        (&self.context.len() +
        &self.function_text.len())/3 //One token is approx. 3 symbols
    }
}
#[derive(Debug)]
pub struct PreparingRequests {
    pub remaining_capacity: usize,
    pub data: Vec<SingleRequestData> 

}
impl PreparingRequests {
    pub fn new() -> PreparingRequests {
        PreparingRequests { 
            remaining_capacity: TOKENS_PER_REQUEST - GoogleGemini::return_prompt().len(), 
            data: vec![] 
        }
    }
    pub fn function_add(&mut self, request_data: SingleRequestData) -> bool {
        if self.remaining_capacity - request_data.size() > 0 {
            self.remaining_capacity = self.remaining_capacity - &request_data.size();
            self.data.push(request_data);
            //println!("capacity left: {}", &self.remaining_capacity);
           return true;
        }
        else{
            println!("exceeded buffer");
            return false;
        }
    }
}
impl Display for PreparingRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:#?}")
    }
}


#[async_trait]
pub trait ReqRes {
    async fn req_res(file_content: String) -> Result<String, ErrorHandling>;
}
#[derive(Debug)]
pub struct GoogleGemini {
    pub preparing_requests: PreparingRequests
} //Req Res = Request Response
#[allow(async_fn_in_trait)]
impl GoogleGemini {
    pub fn new() -> GoogleGemini {
        GoogleGemini { 
            preparing_requests: PreparingRequests { 
                remaining_capacity: TOKENS_PER_MIN/REQUESTS_PER_MIN, 
                data: vec![] 
            } 
        }
    }

    pub async fn req_res(file_content: String, arguments: String) -> Result<String, ErrorHandling> {
        //let api_key = std::env::var("API_KEY_GEMINI").context(StdVarSnafu)?;
        let client = Gemini::new("AIzaSyCqlP-v467ts_yN8POCh1ojijXjd0uRwqc");
        //let args = std::env::var("INPUT_FOR_MODEL")?;
        let res = client
            .generate_content()
            .with_system_prompt(arguments)
            .with_user_message(file_content)
            .execute()
            .await
            .context(GeminiRustSnafu)?;
        Ok(res.text())
    }
    pub fn return_prompt() -> String {
        "The provided data aside from JSON is valid Rust code. Instruction: Locate each function with it's correspondent in JSON, generate /// comment for it and fill it in the types-comment block. Return same JSON structure with filled in comment block for each function. Dismiss.".to_string()
    }
    pub async fn send_batch(batch: Vec<PreparingRequests>)  {
        println!("batch len: {}", batch.len());
        for request in batch {
            let response= GoogleGemini::req_res(request.to_string(), "".to_string()).await.unwrap();
            println!("{:#?}", response);
        }
    }

    // The idea as I see it is: we provide AI Agent with filled out JSON where all the function names are already mapped and 
    // the only goal there is to actually to turn in the JSON and receive it back with written in comments 
    pub fn prepare_batches(&mut self, request: Vec<SingleRequestData>) -> Vec<PreparingRequests>{
        let mut res = Vec::new(); 
        let mut preparing_requests = PreparingRequests::new();
        for data in request {
            if !preparing_requests.function_add(data) {
                //If exceeded remaining capacity
                //Send check if have anything to send
                if preparing_requests.data.len() > 0 {
                    //Send data                 
                    res.push(preparing_requests);
                    preparing_requests = PreparingRequests::new();
                }
            }

        }
        if preparing_requests.data.len() > 0 {
            res.push(preparing_requests);
        }


        res
        

    }
}
