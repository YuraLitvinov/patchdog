use ai_interactions::{YamlRead, return_prompt};
use async_trait::async_trait;
use gemini_rust::Gemini;
use rust_parsing::ErrorHandling;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::ops::Range;
use std::path::PathBuf;
use std::{env::var, fmt::Display, time};
use tracing::{Level, event};
//Theoretical maximum is 250_000, but is highly flawed in a way, that Gemini can 'tear' the response.
//This behavior is explained in call_json_to_rust error case
//Similar issue on https://github.com/googleapis/python-genai/issues/922
//const TOKENS_PER_MIN: usize = 250_000;
//pub const REQUESTS_PER_MIN: usize = 5;
//const TOKENS_PER_REQUEST: usize = TOKENS_PER_MIN / REQUESTS_PER_MIN;
/*

[
    {
        "id": "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
        "fn-name": "main",
        "comment": "bla-bla",
    },
    {
        "id": "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
        "fn-name": "new",
        "comment": "bla-bla",
    }
]

*/

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RawResponse {
    pub uuid: String,
    pub new_comment: String,
}

/*
Here skip serializing occurs because LLM doesn't need to know about external context, such as linerange, filepath.
Although, there is visible clear necessity in including trait information and what are the dependecies which a function would use.
Currently, inclusion of this information is not in the scope.
*/
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SingleFunctionData {
    pub fn_name: String,
    pub function_text: String,
    pub context: Context,
    #[serde(skip_serializing)]
    pub metadata: Metadata,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Context {
    pub class_name: String,
    pub external_dependecies: Vec<String>,
    pub old_comment: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub filepath: PathBuf,
    pub line_range: Range<usize>,
}

impl Context {

    pub fn size(&self) -> usize {
        let mut size_ext = 0;
        for each in &self.external_dependecies {
            size_ext += each.len();
        }
        for each in &self.old_comment {
            size_ext += each.len();
        }
        size_ext
    }
}

impl SingleFunctionData {

    pub fn size(&self) -> usize {
        (self.fn_name.len() + self.context.size() + self.function_text.len()) / 3 //One token is approx. 3 symbols
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Eq)]
pub struct MappedRequest {
    pub remaining_capacity: usize,
    pub data: Vec<Request>,
}

impl MappedRequest {

    pub fn new() -> Result<MappedRequest, ErrorHandling> {
        Ok(MappedRequest {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
            data: Vec::<Request>::new(),
        })
    }

    pub fn function_add(&mut self, request_data: Request) -> bool {
        let size = request_data.data.size();
        if size <= self.remaining_capacity {
            self.data.push(request_data);
            self.remaining_capacity -= size;
            true
        } else {
            false
        }
    }
}

impl Default for MappedRequest {

    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for MappedRequest {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct PreparingRequests {
    pub remaining_capacity: usize,
    pub data: Vec<SingleFunctionData>,
}

impl PreparingRequests {

    pub fn new() -> Result<PreparingRequests, ErrorHandling> {
        Ok(PreparingRequests {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests
                - return_prompt()?.model.len()
                - return_prompt()?.prompt.len(),
            data: vec![],
        })
    }

    pub fn function_add(&mut self, request_data: SingleFunctionData) -> bool {
        let size = request_data.size();
        if size <= self.remaining_capacity {
            self.remaining_capacity -= size;
            self.data.push(request_data);
            true
        } else {
            false
        }
    }
}

impl Default for PreparingRequests {

    fn default() -> Self {
        Self::new().unwrap()
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
    pub preparing_requests: PreparingRequests,
} //Req Res = Request Response

impl Default for GoogleGemini {

    fn default() -> Self {
        Self::new().unwrap()
    }
}

pub fn json_to<T: DeserializeOwned>(val: serde_json::Value) -> T {
    serde_json::from_value(val).unwrap()
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct WaitForTimeout {
    pub prepared_requests: Vec<MappedRequest>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Request {
    pub uuid: String,
    pub data: SingleFunctionData,
}

#[allow(async_fn_in_trait)]
impl GoogleGemini {

    pub fn new() -> Result<GoogleGemini, ErrorHandling> {
        Ok(GoogleGemini {
            preparing_requests: PreparingRequests {
                remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
                data: vec![],
            },
        })
    }

    pub async fn send_batches(request: &Vec<WaitForTimeout>) -> Result<Vec<String>, ErrorHandling> {
        let mut response = vec![];
        let one_minute = time::Duration::from_secs(61);
        for single_request in request {
            for each in &single_request.prepared_requests {
                let as_json = serde_json::to_string_pretty(&each.data)?;
                match GoogleGemini::req_res(&as_json, return_prompt()?).await {
                    //Handling exclusive case, where one of the requests may fail
                    Ok(r) => {
                        response.push(r);
                    }
                    Err(e) => {
                        //error marker
                        return Err(e);
                    }
                }
            }

            if request.len() > 1 {
                tokio::time::sleep(one_minute).await;
            }
        }
        event!(Level::INFO, "exited send_batches");
        Ok(response)
    }

    pub fn request_manager(
        batch: Vec<MappedRequest>,
    ) -> Result<Vec<WaitForTimeout>, ErrorHandling> {
        //Architecture: batch[BIG_NUMBER..len()-1]
        //Next: batch[0..4]
        let mut await_response: Vec<WaitForTimeout> = vec![];
        let request_per_min = return_prompt()?.requests;
        if batch.len() > request_per_min {
            let mut size: usize = batch.len();
            for _ in 1..=batch.len().div_ceil(request_per_min) {
                let mut new_batch: Vec<MappedRequest> = Vec::new();
                //Response where quantity of batches exceed allow per min request count
                //Check for last items in batch
                if size < request_per_min {
                    new_batch.extend_from_slice(&batch[0..size]);
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                    continue;
                } else {
                    new_batch.extend_from_slice(&batch[size.saturating_sub(request_per_min)..size]);
                    size -= request_per_min;
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                }
            }
        } else {
            //Return as normal
            await_response.push(WaitForTimeout {
                prepared_requests: batch,
            });
        }
        Ok(await_response)
    }

    pub async fn req_res(file_content: &str, arguments: YamlRead) -> Result<String, ErrorHandling> {
        let api_key = var("API_KEY_GEMINI")?;
        let model = return_prompt()?.model;
        let client = Gemini::with_model(api_key, model)
            .generate_content()
            .with_system_prompt(arguments.prompt)
            .with_user_message(file_content)
            .execute()
            .await?;
        Ok(client.text())
    }

    pub fn prepare_map(
        &mut self,
        request: Vec<Request>,
    ) -> Result<Vec<MappedRequest>, ErrorHandling> {
        let mut batches: Vec<MappedRequest> = Vec::new();
        let mut mapped_requests = MappedRequest::new()?;
        for data in request {
            if !mapped_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !mapped_requests.data.is_empty() {
                    batches.push(mapped_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                mapped_requests = MappedRequest::new()?;

                // Attempt to push
                if !mapped_requests.function_add(data) {
                    // Here should be handled the case, where single object exceeds token limit
                    //Which is likely would not be possible
                }
            }
        }
        // Last unempty request
        if !mapped_requests.data.is_empty() {
            batches.push(mapped_requests);
        }
        Ok(batches)
    }
}
