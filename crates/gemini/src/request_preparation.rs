use ai_interactions::return_prompt;
use rust_parsing::ErrorHandling;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::path::PathBuf;
use std::{fmt::Display, time};
use tracing::{Level, event};

use crate::bot::{AiRequest, RequestResponseConstruction};
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
    pub external_dependencies: Vec<String>,
    pub old_comment: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub filepath: PathBuf,
    pub line_range: Range<usize>,
}

#[derive(Debug)]
pub struct RequestToAgent {
    pub preparing_requests: PreparingRequests,
} //Req Res = Request Response

impl Context {

/// Calculates an estimated "size" for the current `Context` by summing the character lengths of all strings in its `external_dependencies` and `old_comment` vectors. This size metric is likely used to approximate token usage for LLM requests, helping to manage input limits.
///
/// # Returns
/// A `usize` representing the combined length of all dependency and comment strings, serving as a heuristic for content size.
    pub fn size(&self) -> usize {
        let mut size_ext = 0;
        for each in &self.external_dependencies {
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
  
/// Creates a new `MappedRequest` instance, initializing its `remaining_capacity` based on the LLM's configured tokens per minute divided by allowed requests per minute. This capacity helps in batching individual requests efficiently for LLM processing.
///
/// # Returns
/// A `Result<MappedRequest, ErrorHandling>` containing a new `MappedRequest` instance on success, or an `ErrorHandling` if the application configuration cannot be loaded.
    pub fn new() -> Result<MappedRequest, ErrorHandling> {
        Ok(MappedRequest {
            remaining_capacity: return_prompt()?.llm_settings.tokens
                / return_prompt()?.llm_settings.requests,
            data: Vec::<Request>::new(),
        })
    }

/// Attempts to add a `Request` (containing `SingleFunctionData`) to the current `PreparingRequests` batch. It first checks if the `request_data`'s size fits within the `remaining_capacity` of the batch. If it fits, the request is added, and the capacity is updated.
///
/// # Arguments
/// * `request_data` - The `Request` object, which includes `SingleFunctionData`, to be added to the batch.
///
/// # Returns
/// `true` if the `request_data` was successfully added to the batch, `false` otherwise (e.g., if it exceeds the remaining capacity).
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

/// Provides a default constructor for `PreparingRequests`, by calling its `new()` method and unwrapping the result. This simplifies the creation of a `PreparingRequests` instance when default configuration is acceptable, assuming `new()` will not fail during default initialization.
///
/// # Returns
/// A `PreparingRequests` instance initialized with default or configured capacity.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Display for MappedRequest {
/// Implements the `fmt` trait for the struct, allowing it to be formatted for display. It uses the `Debug` formatter (`{self:#?}`) to write a pretty-printed debug representation of `self` to the formatter. This is useful for detailed debugging output.
///
/// # Arguments
/// * `f` - A mutable reference to a `std::fmt::Formatter`.
///
/// # Returns
/// A `std::fmt::Result` indicating whether the formatting was successful.
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

/// Creates a new `PreparingRequests` instance, calculating its `remaining_capacity` based on LLM token limits and request rates, while also accounting for the lengths of the Gemini model name and the prompt string. The `data` vector for holding function data is initialized as empty.
///
/// # Returns
/// A `Result<PreparingRequests, ErrorHandling>` containing a new `PreparingRequests` instance on success, or an `ErrorHandling` if the application configuration cannot be loaded.
    pub fn new() -> Result<PreparingRequests, ErrorHandling> {
        Ok(PreparingRequests {
            remaining_capacity: return_prompt()?.llm_settings.tokens
                / return_prompt()?.llm_settings.requests
                - return_prompt()?.llm_settings.gemini_model.len()
                - return_prompt()?.prompt.len(),
            data: vec![],
        })
    }

/// Attempts to add a `SingleFunctionData` item to the current `PreparingRequests` batch. The function calculates the size of the incoming data and checks if it fits within the `remaining_capacity`. If successful, the data is added, and the capacity is updated.
///
/// # Arguments
/// * `request_data` - The `SingleFunctionData` object to be added to the batch.
///
/// # Returns
/// `true` if the `SingleFunctionData` was successfully added, `false` otherwise (e.g., if it exceeds the remaining capacity).
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

/// Provides a default constructor for `RequestToAgent`, delegating to the `new()` method and unwrapping the result. This function offers a convenient way to create a `RequestToAgent` instance with default settings, assuming that the `new()` method will always succeed in a default context.
///
/// # Returns
/// A `RequestToAgent` instance initialized with default or configured values.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Implements the `std::fmt::Display` trait for the `PreparingRequests` struct. This enables custom formatting for instances of `PreparingRequests`, specifically by pretty-printing their debug representation (`{self:#?}`) to the provided formatter. It's useful for human-readable output and debugging.
///
/// # Arguments
/// * `f` - A mutable reference to a `std::fmt::Formatter`.
///
/// # Returns
/// A `std::fmt::Result` indicating whether the formatting operation was successful.
impl Display for PreparingRequests {

/// Implements the `std::fmt::Display` trait for the struct, enabling formatted output. This function pretty-prints the debug representation of the instance to the provided formatter, which is particularly useful for logging and debugging purposes.
///
/// # Arguments
/// * `f` - A mutable reference to a `std::fmt::Formatter` where the output will be written.
///
/// # Returns
/// A `std::fmt::Result` indicating whether the formatting operation was successful.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

impl Default for RequestToAgent {

/// Provides a default constructor for `MappedRequest`, which internally calls the `new()` method and unwraps its result. This allows for convenient creation of a `MappedRequest` instance using default settings, assuming that `new()` will not fail during this process.
///
/// # Returns
/// A new `MappedRequest` instance initialized with default or configured capacity.
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Creates a new `RequestToAgent` instance, initializing its `remaining_capacity` for requests based on LLM token and request rate configurations. The `data` vector for preparing requests is initially empty. This function sets up the agent for managing LLM requests.
///
/// # Returns
/// A `Result<RequestToAgent, ErrorHandling>` containing a new `RequestToAgent` instance, or an `ErrorHandling` if configuration loading fails.
#[allow(async_fn_in_trait)]
impl RequestToAgent {

/// Creates a new `RequestToAgent` instance, initializing its `preparing_requests` field. The `remaining_capacity` for requests is calculated based on the LLM's configured tokens and requests per minute, ensuring that agent requests adhere to rate limits.
///
/// # Returns
/// A `Result<RequestToAgent, ErrorHandling>` containing a new `RequestToAgent` instance on success, or an `ErrorHandling` if the application configuration cannot be loaded.
    pub fn new() -> Result<RequestToAgent, ErrorHandling> {
        Ok(RequestToAgent {
            preparing_requests: PreparingRequests {
                remaining_capacity: return_prompt()?.llm_settings.tokens
                    / return_prompt()?.llm_settings.requests,
                data: vec![],
            },
        })
    }

/// Asynchronously sends prepared batches of LLM requests to the configured Large Language Model. It iterates through `WaitForTimeout` batches, serializes each `MappedRequest` within them to JSON, and dispatches it via `AiRequest::switch_llm`. A one-minute delay is introduced between `WaitForTimeout` batches if multiple exist, to respect API rate limits.
///
/// # Arguments
/// * `request` - A reference to a vector of `WaitForTimeout` structs, each containing prepared `MappedRequest` objects.
///
/// # Returns
/// A `Result<Vec<String>, ErrorHandling>` containing a vector of string responses from the LLM, or an `ErrorHandling` if serialization fails, an LLM call fails, or a critical error occurs during processing.
    pub async fn send_batches(request: &Vec<WaitForTimeout>) -> Result<Vec<String>, ErrorHandling> {
        let mut response = vec![];
        let one_minute = time::Duration::from_secs(61);
        for single_request in request {
            for each in &single_request.prepared_requests {
                let as_json = serde_json::to_string_pretty(&each.data)?;
                match AiRequest::switch_llm(&as_json).await {
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
                std::thread::sleep(one_minute);
            }
        }
        event!(Level::INFO, "exited send_batches");
        Ok(response)
    }

/// Manages and batches incoming `MappedRequest` objects into `WaitForTimeout` groups based on configured requests per minute. If the total number of requests exceeds the per-minute limit, it splits them into multiple `WaitForTimeout` batches; otherwise, all requests are placed into a single batch.
///
/// # Arguments
/// * `batch` - A `Vec<MappedRequest>` containing the requests to be managed.
///
/// # Returns
/// A `Result<Vec<WaitForTimeout>, ErrorHandling>`: `Ok(Vec<WaitForTimeout>)` on success, with a vector of `WaitForTimeout` structs, each containing a subset of requests suitable for sending within a time limit, or `Err(ErrorHandling)` if reading configuration fails.
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
        let request_per_min = return_prompt()?.llm_settings.requests;
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

/// Organizes a vector of individual `Request` objects into multiple `MappedRequest` batches, respecting capacity limits. It iteratively adds requests to a current `MappedRequest`. If a request exceeds the current capacity, the filled `MappedRequest` is finalized, a new one is started, and the process continues.
///
/// # Arguments
/// * `request` - A `Vec<Request>` containing the individual requests to be batched.
///
/// # Returns
/// A `Result<Vec<MappedRequest>, ErrorHandling>` containing a vector of `MappedRequest` objects, each holding a subset of requests, or an `ErrorHandling` if an issue occurs during `MappedRequest` initialization.
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
