use crate::binding::{self, changes_from_patch};
use clap::ArgGroup;
use clap::Parser;
use gemini::gemini::Request;
use gemini::gemini::{GoogleGemini, RawResponse, SingleFunctionData, WaitForTimeout};
use regex::Regex;
use rust_parsing::error::ErrorBinding;
use rust_parsing::error::ErrorHandling;
use rust_parsing::file_parsing::REGEX;
use rust_parsing::file_parsing::{FileExtractor, Files};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::{fs, path::PathBuf};
use tracing::{Level, event};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, group(
    ArgGroup::new("path")
        .args(["file_patch"])
        .required(true)
))]
struct Mode {
    #[arg(long)]
    file_patch: PathBuf,
    #[arg(long, num_args=1..14, requires = "file_patch", default_value = "fn")]
    type_rust: Vec<String>,
    #[arg(long, num_args=1..,  requires = "file_patch")]
    name_rust: Vec<String>,
}
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct LinkedResponse {
    data: Request,
    new_comment: String,
}

#[derive(Debug)]
struct ResponseForm {
    data: SingleFunctionData,
    new_comment: String,
}

/// Orchestrates the process of applying a patch via a command-line interface to an agent.
///
/// This asynchronous function performs the following steps:
/// 1. Parses command-line arguments using `Mode::parse()`.
/// 2. Extracts patch data from the specified file using `binding::patch_data_argument`.
/// 3. Transforms the patch data into a vector of requests using `changes_from_patch`, incorporating
///    the specified Rust type and name.
/// 4. If there are requests, it calls an external agent (via the `call` function) with these requests.
/// 5. Collects the responses from the agent.
/// 6. Writes the collected responses to a file using `write_to_file`.
/// Logs progress and informational messages throughout the process.
///
/// # Returns
///
/// - `Ok(())`: An empty `Result` indicating successful execution.
/// - `Err(ErrorBinding)`: An error if any step in the process fails, such as file reading,
///   patch binding, request transformation, agent communication, or file writing.
pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    event!(Level::INFO, "type: {:#?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
    if request.is_empty() {
        event!(Level::INFO, "No requests");
        Ok(())
    }
    else {
        event!(Level::INFO, "Requests length: {}", &request.len());
        let responses_collected = call(request).await?;
        event!(
            Level::INFO,
            "Responses collected: {}",
            responses_collected.len()
        );
        write_to_file(responses_collected)?;
        Ok(())
    }
}

/// Asynchronously processes a batch of `Request`s by sending them to the Google Gemini API,
/// managing responses, and retrying failed requests.
///
/// This function initializes a `GoogleGemini` client, prepares the incoming `request` vector
/// into batches suitable for the API, sends these batches, and then matches the received responses
/// back to the original requests. It collects successful responses into a `Vec<ResponseForm>`.
/// If some requests fail or do not receive a matching response, the function recursively
/// retries processing the remaining requests.
///
/// # Arguments
///
/// * `request` - A `Vec<Request>` containing the requests to be sent to the API.
///
/// # Returns
///
/// - `Ok(Vec<ResponseForm>)`: A `Result` containing a vector of `ResponseForm` instances
///   representing the successful responses, including original request data and new comments.
/// - `Err(ErrorBinding)`: An error if there's an issue with API communication, response matching,
///   or other internal processing failures.
async fn call(request: Vec<Request>) -> Result<Vec<ResponseForm>, ErrorBinding> {
    let mut responses_collected = Vec::new();
    let mut pool_of_requests = HashMap::new();
    request.clone().into_iter().for_each(|each| {
        pool_of_requests.insert(each.uuid, each.data);
    });
    let mut new_buffer = GoogleGemini::new()?;
    let batch = new_buffer.prepare_map(request)?;
    let prepared = GoogleGemini::request_manager(batch)?;
    let response = GoogleGemini::send_batches(&prepared).await?;
    for each in response {
        event!(Level::DEBUG, each);
        let matches = matched_response(&each, &prepared)?;
        for matched in matches {
            let clear_element = pool_of_requests.remove(&matched.data.uuid).ok_or("None");
            match clear_element {
                Ok(ok) => responses_collected.push(ResponseForm {
                    data: ok,
                    new_comment: matched.new_comment,
                }),
                Err(_) => continue,
            }
        }
    }
    if !pool_of_requests.is_empty() {
        event!(
            Level::WARN,
            "Quantity of bad responses: {}",
            pool_of_requests.len()
        );
        let as_vec = pool_of_requests
            .into_iter()
            .map(|(k, v)| Request { uuid: k, data: v })
            .collect();
        let collect_error = Box::pin(call(as_vec)).await?;
        responses_collected.extend(collect_error);
        Ok(responses_collected)
    } else {
        Ok(responses_collected)
    }
}

/// Extracts `RawResponse` objects from a given string using a regular expression.
/// It iterates through all matches of `REGEX` in the input string and attempts to deserialize each match into a `RawResponse` struct.
/// Warnings are logged for any deserialization failures, and those items are skipped.
///
/// # Arguments
///
/// * `response` - A string slice (`&str`) containing the raw response, potentially with multiple JSON objects.
///
/// # Returns
///
/// A `Result<Vec<RawResponse>, ErrorHandling>`:
/// - `Ok(Vec<RawResponse>)`: A vector of successfully parsed `RawResponse` objects.
/// - `Err(ErrorHandling)`: If the regular expression is invalid (unlikely with a hardcoded REGEX) or other internal errors.
pub fn cherrypick_response(response: &str) -> Result<Vec<RawResponse>, ErrorHandling> {
    let re = Regex::new(REGEX).unwrap();
    let mut response_cherrypicked = vec![];
    for cap in re.captures_iter(response) {
        let a = cap.get(0).unwrap().as_str();
        let to_struct = serde_json::from_str::<RawResponse>(a);
        match to_struct {
            Ok(ok) => {
                response_cherrypicked.push(ok);
            }
            Err(e) => {
                event!(Level::WARN, "{e}");
                continue;
            }
        }
    }
    Ok(response_cherrypicked)
}

/// Matches and processes responses from an external agent, handling potential malformed JSON.
/// It first attempts to deserialize the entire `response` string as a `Vec<RawResponse>`.
/// If successful, it performs a cherry-pick extraction and then matches requests to responses.
/// If deserialization fails, it attempts a fallback repair mechanism by cleaning up the response string
/// and then re-attempts cherry-picking and matching.
///
/// # Arguments
///
/// * `response` - A string slice (`&str`) containing the raw response from the agent.
/// * `prepared` - A reference to a `Vec<WaitForTimeout>` representing the original prepared requests.
///
/// # Returns
///
/// A `Result<Vec<LinkedResponse>, ErrorHandling>`:
/// - `Ok(Vec<LinkedResponse>)`: A vector of `LinkedResponse` objects, associating the original request data with the agent's new comment.
/// - `Err(ErrorHandling)`: If response parsing, cherry-picking, or matching fails even after fallback attempts.
fn matched_response(
    response: &str,
    prepared: &Vec<WaitForTimeout>,
) -> Result<Vec<LinkedResponse>, ErrorHandling> {
    match serde_json::from_str::<Vec<RawResponse>>(response) {
        Ok(ok) => {
            let from_reg = cherrypick_response(response)?;
            let res = match_request_response(prepared, &from_reg, &ok)?;
            Ok(res)
        }
        Err(_) => {
            let as_vec = FileExtractor::string_to_vector(response);
            let a = &as_vec[1..as_vec.len() - 1];
            let to_struct = fallback_repair(a.to_vec())?;
            let from_reg = cherrypick_response(response)?;
            let res = match_request_response(prepared, &from_reg, &to_struct)?;
            Ok(res)
        }
    }
}

/// Matches `RawResponse` objects (containing UUIDs and new comments) with the original `WaitForTimeout` prepared requests.
/// It uses HashMaps to efficiently combine and deduplicate responses, prioritizing `cherrypicked_response`.
/// If the lengths of `single_set` and `multi_set` are equal, it uses the `singlerun_response` for matching.
/// Otherwise, it combines the sets and then matches using the combined `RawResponse` vector.
///
/// # Arguments
///
/// * `prepared` - A reference to a `Vec<WaitForTimeout>` containing the original prepared requests, which include `MappedRequest` and `SingleFunctionData`.
/// * `cherrypicked_response` - A slice of `RawResponse` objects obtained via cherry-picking, often representing a subset of valid responses.
/// * `singlerun_response` - A slice of `RawResponse` objects from a single run, which might contain more comprehensive or less filtered responses.
///
/// # Returns
///
/// A `Result<Vec<LinkedResponse>, ErrorHandling>`:
/// - `Ok(Vec<LinkedResponse>)`: A vector of `LinkedResponse` objects, where each links an original `SingleFunctionData` to its corresponding `new_comment`.
/// - `Err(ErrorHandling)`: If any internal error occurs during data processing (e.g., hash map operations).
fn match_request_response(
    prepared: &Vec<WaitForTimeout>,
    cherrypicked_response: &[RawResponse],
    singlerun_response: &[RawResponse],
) -> Result<Vec<LinkedResponse>, ErrorHandling> {
    let single_set: HashMap<String, String> = cherrypicked_response
        .iter()
        .map(|each| (each.uuid.clone(), each.new_comment.clone()))
        .collect();
    let mut multi_set: HashMap<String, String> = singlerun_response
        .iter()
        .map(|each| (each.uuid.clone(), each.new_comment.clone()))
        .collect();
    single_set.clone().into_iter().for_each(|(k, v)| {
        multi_set.insert(k, v);
    });
    if single_set.len() == multi_set.len() {
        let collected = matching(prepared, singlerun_response);
        Ok(collected)
    } else {
        let combined = single_set
            .iter()
            .filter_map(|(k, v)| multi_set.remove(k).map(|_| (k.clone(), v.clone())))
            .collect::<HashMap<String, String>>()
            .into_iter()
            .map(|(k, v)| RawResponse {
                uuid: k,
                new_comment: v,
            })
            .collect::<Vec<RawResponse>>();
        let collected = matching(prepared, &combined);
        Ok(collected)
    }
}

/// Matches and links `RawResponse` objects from an agent's response to their corresponding `SingleFunctionData` within prepared requests.
/// It iterates through all individual requests contained within the `prepared` batches.
/// For each such request, it tries to find a matching `RawResponse` based on UUID.
/// If a match is found, a `LinkedResponse` struct is created, associating the original data with the new comment.
///
/// # Arguments
///
/// * `prepared` - A reference to a `Vec<WaitForTimeout>` containing the structured, prepared batches of requests.
/// * `response` - A slice of `RawResponse` objects, which are the processed responses from the AI agent.
///
/// # Returns
///
/// A `Vec<LinkedResponse>` containing all successfully matched original request data with their new comments.
fn matching(prepared: &Vec<WaitForTimeout>, response: &[RawResponse]) -> Vec<LinkedResponse> {
    let mut matched = vec![];
    for prepare in prepared {
        for req in &prepare.prepared_requests {
            for each in &req.data {
                if let Some(found) = response.iter().find(|item| item.uuid == each.uuid) {
                    matched.push(LinkedResponse {
                        data: each.to_owned(),
                        new_comment: found.new_comment.to_string(),
                    });
                }
            }
        }
    }
    matched
}

/// Attempts to repair and deserialize a potentially malformed JSON output from an external agent.
/// This function iterates by progressively removing lines from the end of the input vector,
/// then attempts to append a closing JSON delimiter (`}]`) and deserialize the modified string into a `Vec<RawResponse>`.
/// It returns the first successful deserialization result.
/// This is a fallback mechanism for cases where the agent's JSON response is truncated or incomplete.
///
/// # Arguments
///
/// * `output` - A `Vec<String>` representing the lines of the raw, potentially broken, JSON output.
///
/// # Returns
///
/// A `Result<Vec<RawResponse>, ErrorHandling>`:
/// - `Ok(Vec<RawResponse>)`: If a valid `Vec<RawResponse>` can be parsed after a repair attempt.
/// - `Err(ErrorHandling::CouldNotGetLine)`: If no valid JSON can be parsed even after all repair attempts.
fn fallback_repair(output: Vec<String>) -> Result<Vec<RawResponse>, ErrorHandling> {
    let mut clone_out = output;
    for _ in 0..clone_out.len() {
        clone_out.pop();
        let mut clone_clone = clone_out.clone();
        //Fixing broken delimiters in returned JSON here
        clone_clone.push("}]".to_string());
        let _ = match serde_json::from_str::<Vec<RawResponse>>(&clone_clone.join("\n")) {
            Ok(res) => {
                return Ok(res);
            }
            Err(_) => {
                continue;
            }
        };
    }
    event!(Level::WARN, "Here");
    Err(ErrorHandling::CouldNotGetLine)
}

/// Writes the `new_comment` from each `ResponseForm` back into its corresponding file.
/// The responses are sorted by `line_range.start` in descending order to prevent issues with line index shifts during insertion.
/// For each response, it reads the file, converts its content to a vector of strings, inserts the new comment at the specified line index,
/// and then writes the modified content back to the file.
///
/// # Arguments
///
/// * `response` - A `Vec<ResponseForm>` containing the responses with original file data and new comments.
///
/// # Returns
///
/// An `Ok(())` on successful writing to all files.
/// An `Err(ErrorHandling)` if any file operation (reading, creating, writing) fails.
fn write_to_file(response: Vec<ResponseForm>) -> Result<(), ErrorHandling> {
    let mut response = response;
    response.sort_by(|a, b| {
        b.data
            .metadata
            .line_range
            .start
            .cmp(&a.data.metadata.line_range.start)
    });
    event!(Level::INFO, "Quantity of responses: {}", response.len());
    //Typical representation of file as vector of lines
    for each in response {
        let path = each.data.metadata.filepath;
        let file = fs::read_to_string(&path)?;
        let as_vec = FileExtractor::string_to_vector(&file);
        FileExtractor::write_to_vecstring(
            path,
            as_vec,
            each.data.metadata.line_range.start,
            each.new_comment,
        )?;
    }
    Ok(())
}
