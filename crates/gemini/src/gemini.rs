use ai_interactions::return_prompt;
use async_trait::async_trait;
use gemini_rust::Gemini;
use rust_parsing::error::{ErrorBinding, SerdeSnafu};
use rust_parsing::{ErrorHandling, error::GeminiRustSnafu, error::StdVarSnafu};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use snafu::ResultExt;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::{env::var, fmt::Display, time};
//Theoretical maximum is 250_000, but is highly flawed in a way, that Gemini can 'tear' the response.
//This behavior is explained in call_json_to_rust error case
//Similar issue on https://github.com/googleapis/python-genai/issues/922
const TOKENS_PER_MIN: usize = 250_000;
pub const REQUESTS_PER_MIN: usize = 5;
const TOKENS_PER_REQUEST: usize = TOKENS_PER_MIN / REQUESTS_PER_MIN;
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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Response {
    pub uuid: String,
    pub fn_name: String,
    pub new_comment: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SingleFunctionData {
    pub fn_name: String,
    pub function_text: String,
    #[serde(skip_serializing)]
    pub context: ContextData,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ContextData {
    pub class_name: String,
    pub filepath: PathBuf,
    pub external_dependecies: Vec<String>,
    pub old_comment: Vec<String>,
    pub line_range: Range<usize>,
}
impl ContextData {
/// Calculates the size of a `ContextData` struct.
///
/// # Returns
///
/// The size of the struct.
    pub fn size(&self) -> usize {
        let mut size_ext = 0;
        for each in &self.external_dependecies {
            size_ext += each.len();
        }
        for each in &self.old_comment {
            size_ext += each.len();
        }
        self.class_name.len() + self.filepath.to_str().unwrap().len() + size_ext + self.line_range.len()
    }
}

impl SingleFunctionData {
/// Calculates the approximate size of a `SingleFunctionData` struct in tokens.
///
/// # Returns
///
/// The approximate size in tokens.
    pub fn size(&self) -> usize {
        (self.fn_name.len() + self.context.size() + self.function_text.len()) / 3 //One token is approx. 3 symbols
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Eq)]
pub struct MappedRequest {
    pub remaining_capacity: usize,
    pub data: HashMap<String, SingleFunctionData>,
}

impl MappedRequest {
/// Creates a new `MappedRequest` struct with the remaining capacity set to `TOKENS_PER_REQUEST` and an empty HashMap.
///
/// # Returns
///
/// A new `MappedRequest` struct.
    pub fn new() -> MappedRequest {
        MappedRequest {
            remaining_capacity: TOKENS_PER_REQUEST,
            data: HashMap::new(),
        }
    }
/// Adds a `SingleFunctionData` struct to the internal data HashMap, generating a new UUID for each entry.
///
/// # Arguments
///
/// * `request_data`: The `SingleFunctionData` struct to add.
///
/// # Returns
///
/// `true` if the data was added successfully, `false` otherwise.
    pub fn function_add(&mut self, request_data: SingleFunctionData) -> bool {
        let size = request_data.size();
        if size <= self.remaining_capacity {
            self.data
                .insert(uuid::Uuid::new_v4().to_string(), request_data);
            self.remaining_capacity -= size;
            true
        } else {
            false
        }
    }
}

impl Default for MappedRequest {
/// Creates a default instance of the struct.
///
/// # Returns
///
/// A new instance of the struct.
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MappedRequest {
/// Formats the value using the given formatter.
///
/// # Arguments
///
/// * `f`: The formatter to use.
///
/// # Returns
///
/// A `Result` indicating whether formatting was successful.
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
/// Creates a new `PreparingRequests` struct with the remaining capacity set to `TOKENS_PER_REQUEST` minus the length of the return prompt and an empty data vector.
///
/// # Returns
///
/// A new `PreparingRequests` struct.
    pub fn new() -> PreparingRequests {
        PreparingRequests {
            remaining_capacity: TOKENS_PER_REQUEST - return_prompt().len(),
            data: vec![],
        }
    }
/// Adds a `SingleFunctionData` struct to the internal data vector if there is enough remaining capacity.
///
/// # Arguments
///
/// * `request_data`: The `SingleFunctionData` struct to add.
///
/// # Returns
///
/// `true` if the data was added successfully, `false` otherwise.
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
/// Creates a default instance of the struct.
///
/// # Returns
///
/// A new instance of the struct.
    fn default() -> Self {
        Self::new()
    }
}

impl Display for PreparingRequests {
/// Formats the value using the given formatter.
///
/// # Arguments
///
/// * `f`: The formatter to use.
///
/// # Returns
///
/// A `Result` indicating whether formatting was successful.
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
/// Creates a default instance of the struct.
///
/// # Returns
///
/// A new instance of the struct.
    fn default() -> Self {
        Self::new()
    }
}
/// Converts a `serde_json::Value` to a specified type.
///
/// # Arguments
///
/// * `val`: The `serde_json::Value` to convert.
///
/// # Returns
///
/// The converted value.
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
    uuid: String,
    data: SingleFunctionData,
}

#[allow(async_fn_in_trait)]
impl GoogleGemini {
/// Creates a new `GoogleGemini` struct with the remaining capacity set to `TOKENS_PER_MIN / REQUESTS_PER_MIN` and an empty data vector.
///
/// # Returns
///
/// A new `GoogleGemini` struct.
    pub fn new() -> GoogleGemini {
        GoogleGemini {
            preparing_requests: PreparingRequests {
                remaining_capacity: TOKENS_PER_MIN / REQUESTS_PER_MIN,
                data: vec![],
            },
        }
    }
/// Sends batches of requests to the Google Gemini API.
///
/// # Arguments
///
/// * `request`: A vector of `WaitForTimeout` structs representing the requests to send.
///
/// # Returns
///
/// A `Result` containing a vector of strings representing the responses, or an `ErrorHandling` if any error occurred.
    pub async fn send_batches(request: &Vec<WaitForTimeout>) -> Result<Vec<String>, ErrorHandling> {
        let mut response = vec![];
        let one_minute = time::Duration::from_secs(61);
        for single_request in request {
            for each in &single_request.prepared_requests {
                let fmt = &each.data;
                let mut vec = vec![];
                for (val, each) in fmt {
                    vec.push(Request {
                        uuid: val.clone(),
                        data: each.clone(),
                    });
                }
                let as_json = serde_json::to_string_pretty(&vec).context(SerdeSnafu)?;
                match GoogleGemini::req_res(&as_json, return_prompt()).await {
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
        println!("exited send_batches");
        Ok(response)
    }

    pub fn assess_batch_readiness(
        batch: Vec<MappedRequest>,
    ) -> Result<Vec<WaitForTimeout>, ErrorBinding> {
        //Architecture: batch[BIG_NUMBER..len()-1]
        //Next: batch[0..4]
        let mut await_response: Vec<WaitForTimeout> = vec![];
        if batch.len() > REQUESTS_PER_MIN {
            let mut size: usize = batch.len();
            for _ in 1..=batch.len().div_ceil(REQUESTS_PER_MIN) {
                let mut new_batch: Vec<MappedRequest> = Vec::new();
                //Response where quantity of batches exceed allow per min request count
                //Check for last items in batch
                if size < REQUESTS_PER_MIN {
                    new_batch.extend_from_slice(&batch[0..size]);
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                    continue;
                } else {
                    new_batch
                        .extend_from_slice(&batch[size.saturating_sub(REQUESTS_PER_MIN)..size]);
                    size -= REQUESTS_PER_MIN;
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

    pub async fn req_res(file_content: &str, arguments: &str) -> Result<String, ErrorHandling> {
        dotenv::from_path(".env").unwrap();
        let api_key = var("API_KEY_GEMINI").context(StdVarSnafu)?;
        let model = var("GEMINI_MODEL").context(StdVarSnafu)?;
        println!("{} {}", &api_key[0..3], model);
        let client = Gemini::with_model(api_key, model)
            .generate_content()
            .with_system_prompt(arguments)
            .with_user_message(file_content)
            .execute()
            .await
            .context(GeminiRustSnafu)?;

        Ok(client.text())
    }

    // The idea as I see it is: we provide AI Agent with filled out JSON where all the function names are already mapped and
    // the only goal there is to actually to turn in the JSON and receive it back with written in comments
    pub fn prepare_batches(
        &mut self,
        request: Vec<SingleFunctionData>,
    ) -> Result<Vec<PreparingRequests>, ErrorHandling> {
        let mut batches: Vec<PreparingRequests> = Vec::new();
        let mut preparing_requests = PreparingRequests::new();
        for data in request {
            if !preparing_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !preparing_requests.data.is_empty() {
                    batches.push(preparing_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                preparing_requests = PreparingRequests::new();

                // Attempt to push
                if !preparing_requests.function_add(data) {
                    // Here should be handled the case, where single object exceeds token limit
                    //Which is likely would not be possible
                }
            }
        }

        // Last unempty request
        if !preparing_requests.data.is_empty() {
            batches.push(preparing_requests);
        }
        Ok(batches)
    }

    pub fn prepare_map(
        &mut self,
        request: Vec<SingleFunctionData>,
    ) -> Result<Vec<MappedRequest>, ErrorHandling> {
        let mut batches: Vec<MappedRequest> = Vec::new();
        let mut mapped_requests = MappedRequest::new();
        for data in request {
            if !mapped_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !mapped_requests.data.is_empty() {
                    batches.push(mapped_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                mapped_requests = MappedRequest::new();

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
