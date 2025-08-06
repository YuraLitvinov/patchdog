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

/// Calculates the total length of all external dependencies and old comments within the struct.
/// This is typically used to estimate the size of contextual data.
///
/// # Arguments
///
/// * `&self` - A reference to the current instance of the struct containing `external_dependecies` and `old_comment` fields.
///
/// # Returns
///
/// A `usize` representing the sum of the lengths of all strings in `external_dependecies` and `old_comment`.
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

/// Estimates the size of the current struct instance based on the lengths of its `fn_name`, `context`, and `function_text` fields.
/// The total length is divided by 3, assuming an average of 3 symbols per token for estimation purposes.
///
/// # Arguments
///
/// * `&self` - A reference to the current instance of the struct.
///
/// # Returns
///
/// A `usize` representing the estimated token size of the struct.
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

/// Creates a new `MappedRequest` instance, initializing its remaining capacity based on token and request limits read from configuration.
/// The capacity is calculated as `tokens_per_minute / requests_per_minute`.
///
/// # Returns
///
/// A `Result<MappedRequest, ErrorHandling>`:
/// - `Ok(MappedRequest)`: A new `MappedRequest` with calculated `remaining_capacity` and an empty `data` vector.
/// - `Err(ErrorHandling)`: If reading configuration (via `return_prompt()`) fails.
    pub fn new() -> Result<MappedRequest, ErrorHandling> {
        Ok(MappedRequest {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
            data: Vec::<Request>::new(),
        })
    }

/// Attempts to add a `Request` to the internal data vector if its size does not exceed the `remaining_capacity`.
/// If the request fits, it's added, and the `remaining_capacity` is updated.
///
/// # Arguments
///
/// * `&mut self` - A mutable reference to the `MappedRequest` instance.
/// * `request_data` - The `Request` to be added.
///
/// # Returns
///
/// A `bool`:
/// - `true`: If the `request_data` was successfully added.
/// - `false`: If the `request_data` exceeds the `remaining_capacity` and could not be added.
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

/// Provides a default instance of `Self` by calling the `new()` constructor and unwrapping its result.
/// This implementation assumes that `new()` will always succeed in a default context.
///
/// # Returns
///
/// A `Self` instance, initialized via `Self::new().unwrap()`.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for MappedRequest {

/// Implements the `fmt` trait for the struct, allowing it to be formatted for display.
/// It uses the `Debug` formatter (`{self:#?}`) to write a pretty-printed debug representation of `self` to the formatter.
///
/// # Arguments
///
/// * `&self` - A reference to the struct instance.
/// * `f` - A mutable reference to a `std::fmt::Formatter`.
///
/// # Returns
///
/// A `std::fmt::Result` indicating whether the formatting was successful.
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

/// Creates a new `PreparingRequests` instance.
/// It initializes the `remaining_capacity` by calculating it based on token and request limits, subtracting the lengths of the model name and prompt from the configuration.
/// The `data` vector is initialized as empty.
///
/// # Returns
///
/// A `Result<PreparingRequests, ErrorHandling>`:
/// - `Ok(PreparingRequests)`: A new instance with calculated `remaining_capacity` and an empty `data` vector.
/// - `Err(ErrorHandling)`: If reading configuration (via `return_prompt()`) fails.
    pub fn new() -> Result<PreparingRequests, ErrorHandling> {
        Ok(PreparingRequests {
            remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests
                - return_prompt()?.model.len()
                - return_prompt()?.prompt.len(),
            data: vec![],
        })
    }

/// Attempts to add a `SingleFunctionData` item to the internal `data` vector if its size does not exceed the `remaining_capacity`.
/// If the item fits, it's added, and the `remaining_capacity` is reduced by the item's size.
///
/// # Arguments
///
/// * `&mut self` - A mutable reference to the `PreparingRequests` instance.
/// * `request_data` - The `SingleFunctionData` item to be added.
///
/// # Returns
///
/// A `bool`:
/// - `true`: If the `request_data` was successfully added.
/// - `false`: If the `request_data` exceeds the `remaining_capacity` and could not be added.
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

/// Provides a default instance of `Self` by calling the `new()` constructor and unwrapping its result.
/// This implementation assumes that `new()` will always succeed in a default context.
///
/// # Returns
///
/// A `Self` instance, initialized via `Self::new().unwrap()`.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for PreparingRequests {

/// Implements the `fmt` trait for the struct, allowing it to be formatted for display.
/// It uses the `Debug` formatter (`{self:#?}`) to write a pretty-printed debug representation of `self` to the formatter.
///
/// # Arguments
///
/// * `&self` - A reference to the struct instance.
/// * `f` - A mutable reference to a `std::fmt::Formatter`.
///
/// # Returns
///
/// A `std::fmt::Result` indicating whether the formatting was successful.
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

/// Provides a default instance of `Self` by calling the `new()` constructor and unwrapping its result.
/// This implementation assumes that `new()` will always succeed in a default context.
///
/// # Returns
///
/// A `Self` instance, initialized via `Self::new().unwrap()`.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Deserializes a `serde_json::Value` into a specified type `T` that implements `DeserializeOwned`.
/// This function is a convenience wrapper for `serde_json::from_value` and unwraps the result,
/// meaning it will panic if deserialization fails.
///
/// # Type Parameters
///
/// * `T`: The target type to deserialize into, which must implement `serde::de::DeserializeOwned`.
///
/// # Arguments
///
/// * `val` - The `serde_json::Value` to be deserialized.
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

/// Creates a new `GoogleGemini` instance, initializing its `preparing_requests` field.
/// The `remaining_capacity` for `preparing_requests` is calculated based on token and request limits retrieved from configuration.
///
/// # Returns
///
/// A `Result<GoogleGemini, ErrorHandling>`:
/// - `Ok(GoogleGemini)`: A new instance with `preparing_requests` initialized.
/// - `Err(ErrorHandling)`: If reading configuration (via `return_prompt()`) fails.
    pub fn new() -> Result<GoogleGemini, ErrorHandling> {
        Ok(GoogleGemini {
            preparing_requests: PreparingRequests {
                remaining_capacity: return_prompt()?.tokens / return_prompt()?.requests,
                data: vec![],
            },
        })
    }

/// Asynchronously sends batches of prepared requests to the Google Gemini API.
/// It iterates through `WaitForTimeout` objects, converts `SingleFunctionData` to JSON, and sends each as a request.
/// If there's more than one request batch, it introduces a one-minute delay between batches to respect API rate limits.
/// Errors during any request are immediately propagated.
///
/// # Arguments
///
/// * `request` - A reference to a `Vec<WaitForTimeout>` containing the batches of requests to send.
///
/// # Returns
///
/// A `Result<Vec<String>, ErrorHandling>`:
/// - `Ok(Vec<String>)`: A vector of `String`s, where each string is the successful response text from the API.
/// - `Err(ErrorHandling)`: If any request fails or an error occurs during JSON serialization or API communication.
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

/// Manages and batches incoming `MappedRequest` objects into `WaitForTimeout` groups based on configured requests per minute.
/// If the total number of requests exceeds the per-minute limit, it splits them into multiple `WaitForTimeout` batches.
/// Otherwise, all requests are placed into a single batch.
///
/// # Arguments
///
/// * `batch` - A `Vec<MappedRequest>` containing the requests to be managed.
///
/// # Returns
///
/// A `Result<Vec<WaitForTimeout>, ErrorHandling>`:
/// - `Ok(Vec<WaitForTimeout>)`: A vector of `WaitForTimeout` structs, each containing a subset of requests suitable for sending within a time limit.
/// - `Err(ErrorHandling)`: If reading configuration (via `return_prompt()`) fails.
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

/// Sends a content generation request to the Google Gemini API using the provided file content and configuration arguments.
/// It retrieves the API key from environment variables, initializes the Gemini client with the specified model, and sends the user message along with a system prompt.
///
/// # Arguments
///
/// * `file_content` - A string slice (`&str`) representing the content to be sent as a user message.
/// * `arguments` - A `YamlRead` struct containing configuration details such as the system prompt.
///
/// # Returns
///
/// A `Result<String, ErrorHandling>`:
/// - `Ok(String)`: The text content of the response from the Gemini API.
/// - `Err(ErrorHandling)`: If the API key is not found, API communication fails, or any other error occurs during the process.
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

/// Prepares a vector of `Request` objects into batches of `MappedRequest` instances.
/// It iterates through the incoming requests and attempts to add them to the current `MappedRequest` batch.
/// If a request does not fit into the current batch (due to capacity limits), the current batch is finalized and added to `batches`,
/// and a new `MappedRequest` is initialized for subsequent requests.
///
/// # Arguments
///
/// * `&mut self` - A mutable reference to the `GoogleGemini` instance, specifically its `preparing_requests` field.
/// * `request` - A `Vec<Request>` containing the individual requests to be mapped into batches.
///
/// # Returns
///
/// A `Result<Vec<MappedRequest>, ErrorHandling>`:
/// - `Ok(Vec<MappedRequest>)`: A vector of `MappedRequest` batches, each filled up to its capacity limit.
/// - `Err(ErrorHandling)`: If creating a new `MappedRequest` fails.
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
