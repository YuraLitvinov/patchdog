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
    /// Calculates the combined length (in characters) of the strings within the `external_dependecies` and `old_comment` vectors of the `ContextData` struct.
    /// This function provides an approximate measure of the data contained within these fields.
    ///
    /// # Returns
    ///
    /// A `usize` representing the total calculated size.
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
        size_ext
    }
}

impl SingleFunctionData {
    /// Estimates the approximate size of a `SingleFunctionData` struct in tokens. The calculation is based on the combined character length of the function's name, context data size, and function text, divided by an assumed average of 3 symbols per token.
    ///
    /// # Returns
    ///
    /// A `usize` representing the estimated size in tokens.
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
    pub data: Vec<Request>,
}

impl MappedRequest {
    /// Constructs a new `MappedRequest` instance. It initializes the `remaining_capacity` based on environment variables `TOKENS_PER_MIN` and `REQUESTS_PER_MIN` (representing the token limit per request), and sets up an empty vector to hold request data.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `MappedRequest` struct, or an `ErrorHandling` if environment variables cannot be parsed.
    /// Creates a new `MappedRequest` struct with the remaining capacity set to `TOKENS_PER_REQUEST` and an empty HashMap.
    ///
    /// # Returns
    ///
    /// A new `MappedRequest` struct.
    pub fn new() -> Result<MappedRequest, ErrorHandling> {
        Ok(MappedRequest {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
            data: Vec::<Request>::new(),
        })
    }
    /// Attempts to add a `Request` object to the internal data collection of the `MappedRequest`.
    /// The `Request` is added only if its calculated size does not exceed the `remaining_capacity`.
    /// If successful, the `remaining_capacity` is reduced by the size of the added request.
    ///
    /// # Arguments
    ///
    /// * `request_data`: The `Request` struct to be added.
    ///
    /// # Returns
    ///
    /// `true` if the `request_data` was successfully added (i.e., there was enough capacity), otherwise `false`.
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
    /// Creates a default instance of the `MappedRequest` struct by calling its `new` constructor and unwrapping the result.
    /// This implementation assumes `new` will always succeed in a default context.
    ///
    /// # Returns
    ///
    /// A new, default `MappedRequest` instance.
    /// Creates a default instance of the struct.
    ///
    /// # Returns
    ///
    /// A new instance of the struct.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for MappedRequest {
    /// Implements the `Display` trait for the current struct, formatting its debug representation into the provided formatter.
    /// This allows for easy printing of the struct's contents using `println!("{}", my_struct)` or similar.
    ///
    /// # Arguments
    ///
    /// * `f`: A mutable reference to the `std::fmt::Formatter` to write into.
    ///
    /// # Returns
    ///
    /// A `std::fmt::Result` indicating success or failure of the formatting operation.
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
    /// Constructs a new `PreparingRequests` instance. It initializes the `remaining_capacity` by calculating the token limit per request from environment variables (`TOKENS_PER_MIN`, `REQUESTS_PER_MIN`) and subtracting the length of the default prompt. An empty vector is set up to store data.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `PreparingRequests` struct, or an `ErrorHandling` if environment variables cannot be parsed or the prompt cannot be retrieved.
    /// Creates a new `PreparingRequests` struct with the remaining capacity set to `TOKENS_PER_REQUEST` minus the length of the return prompt and an empty data vector.
    ///
    /// # Returns
    ///
    /// A new `PreparingRequests` struct.
    pub fn new() -> Result<PreparingRequests, ErrorHandling> {
        Ok(PreparingRequests {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests
                - return_prompt()?.model.len()
                - return_prompt()?.prompt.len(),
            data: vec![],
        })
    }
    /// Attempts to add a `SingleFunctionData` object to the `PreparingRequests`'s internal data vector.
    /// The data is added only if its calculated size does not exceed the `remaining_capacity`.
    /// If successful, the `remaining_capacity` is reduced by the size of the added data.
    ///
    /// # Arguments
    ///
    /// * `request_data`: The `SingleFunctionData` struct to be added.
    ///
    /// # Returns
    ///
    /// `true` if the `request_data` was successfully added (i.e., there was enough capacity), otherwise `false`.
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
    /// Creates a default instance of the `PreparingRequests` struct by calling its `new` constructor and unwrapping the result.
    /// This implementation assumes `new` will always succeed in a default context.
    ///
    /// # Returns
    ///
    /// A new, default `PreparingRequests` instance.
    /// Creates a default instance of the struct.
    ///
    /// # Returns
    ///
    /// A new instance of the struct.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for PreparingRequests {
    /// Implements the `Display` trait for the current struct, formatting its debug representation into the provided formatter.
    /// This allows for easy printing of the struct's contents using `println!("{}", my_struct)` or similar.
    ///
    /// # Arguments
    ///
    /// * `f`: A mutable reference to the `std::fmt::Formatter` to write into.
    ///
    /// # Returns
    ///
    /// A `std::fmt::Result` indicating success or failure of the formatting operation.
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
    /// Creates a default instance of the `GoogleGemini` struct by calling its `new` constructor and unwrapping the result.
    /// This implementation assumes `new` will always succeed in a default context.
    ///
    /// # Returns
    ///
    /// A new, default `GoogleGemini` instance.
    /// Creates a default instance of the struct.
    ///
    /// # Returns
    ///
    /// A new instance of the struct.
    fn default() -> Self {
        Self::new().unwrap()
    }
}
/// Converts a `serde_json::Value` into an instance of a specified type `T` that implements `DeserializeOwned`.
/// This function will panic if the conversion fails, making it suitable for cases where the input `serde_json::Value` is guaranteed to be compatible with `T`.
///
/// # Arguments
///
/// * `val`: The `serde_json::Value` to be converted.
///
/// # Returns
///
/// An instance of type `T`.
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
    /// Constructs a new `GoogleGemini` instance, initializing its internal `preparing_requests` field.
    /// The `remaining_capacity` for `preparing_requests` is calculated based on environment variables `TOKENS_PER_MIN` and `REQUESTS_PER_MIN`, and its `data` vector is initialized as empty.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `GoogleGemini` struct, or an `ErrorHandling` if environment variables cannot be parsed.
    /// Creates a new `GoogleGemini` struct with the remaining capacity set to `TOKENS_PER_MIN / REQUESTS_PER_MIN` and an empty data vector.
    ///
    /// # Returns
    ///
    /// A new `GoogleGemini` struct.
    pub fn new() -> Result<GoogleGemini, ErrorHandling> {
        Ok(GoogleGemini {
            preparing_requests: PreparingRequests {
                remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
                data: vec![],
            },
        })
    }
    /// Sends prepared batches of requests to the Google Gemini API asynchronously. Each request within a `WaitForTimeout` batch is converted to JSON and sent.
    /// If multiple `WaitForTimeout` batches exist, a one-minute delay is introduced between sending each batch to respect API rate limits.
    /// Errors during any request will cause the function to return immediately.
    ///
    /// # Arguments
    ///
    /// * `request`: A reference to a `Vec<WaitForTimeout>` containing the requests to be sent.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<String>` of successful API responses, or an `ErrorHandling` if an error occurs during JSON serialization, API calls, or I/O.
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

    /// Assesses the readiness of a collection of `MappedRequest` batches by dividing them into smaller `WaitForTimeout` structs.
    /// This division is based on the `REQUESTS_PER_MIN` environment variable, ensuring that the number of requests in each `WaitForTimeout` batch does not exceed the allowed rate limit.
    ///
    /// # Arguments
    ///
    /// * `batch`: A `Vec<MappedRequest>` representing the full set of requests to be processed.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<WaitForTimeout>` of prepared batches, or an `ErrorHandling` if `REQUESTS_PER_MIN` cannot be parsed from environment variables.
    pub fn assess_batch_readiness(
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

    /// Sends a content generation request to the Google Gemini API.
    /// It retrieves the API key and model name from environment variables, then constructs and executes a content generation request with a system prompt and user message.
    ///
    /// # Arguments
    ///
    /// * `file_content`: A string slice containing the user message/content for the API request.
    /// * `arguments`: A string slice containing the system prompt for the API request.
    ///
    /// # Returns
    ///
    /// An asynchronous `Result` containing the generated text response as a `String`, or an `ErrorHandling` if API key/model retrieval, request execution, or response processing fails.
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

    // The idea as I see it is: we provide AI Agent with filled out JSON where all the function names are already mapped and
    // the only goal there is to actually to turn in the JSON and receive it back with written in comments
    /// Organizes a vector of `SingleFunctionData` requests into a series of `PreparingRequests` batches.
    /// It iterates through the requests, adding them to the current `PreparingRequests` instance until its capacity is full.
    /// Once a batch is full or all requests are processed, the `PreparingRequests` instance is pushed to the `batches` vector, and a new `PreparingRequests` is initialized.
    ///
    /// # Arguments
    ///
    /// * `request`: A `Vec<SingleFunctionData>` representing the individual requests to be batched.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<PreparingRequests>` of the organized batches, or an `ErrorHandling` if an error occurs during `PreparingRequests` initialization.
    pub fn prepare_batches(
        &mut self,
        request: Vec<SingleFunctionData>,
    ) -> Result<Vec<PreparingRequests>, ErrorHandling> {
        let mut batches: Vec<PreparingRequests> = Vec::new();
        let mut preparing_requests = PreparingRequests::new()?;
        for data in request {
            if !preparing_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !preparing_requests.data.is_empty() {
                    batches.push(preparing_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                preparing_requests = PreparingRequests::new()?;

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

    /// Organizes a vector of `Request` structs into a series of `MappedRequest` batches.
    /// It iterates through the requests, attempting to add each to the current `MappedRequest` instance. If the current batch's capacity is exceeded, the full batch is pushed to the `batches` vector, and a new `MappedRequest` is initialized for subsequent requests.
    ///
    /// # Arguments
    ///
    /// * `request`: A `Vec<Request>` representing the individual requests to be mapped into batches.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<MappedRequest>` of the organized batches, or an `ErrorHandling` if an error occurs during `MappedRequest` initialization.
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
