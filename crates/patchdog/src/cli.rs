use crate::binding::{self, changes_from_patch};
use clap::ArgGroup;
use clap::Parser;
use gemini::request_preparation::Request;
use gemini::request_preparation::RequestToAgent;
use gemini::request_preparation::{RawResponse, SingleFunctionData, WaitForTimeout};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use regex::Regex;
use rust_parsing::error::ErrorBinding;
use rust_parsing::error::ErrorHandling;
use rust_parsing::file_parsing::REGEX;
use rust_parsing::file_parsing::{FileExtractor, Files};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::{fs, path::PathBuf, env};
use tracing::{Level, event};
use snafu::ResultExt;
use rust_parsing::error::InvalidIoOperationsSnafu;

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
#[allow(dead_code)]
pub struct ResponseForm {
    data: SingleFunctionData,
    new_comment: String,
}

pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    //Mode accepts type and name of the object for the sake of debugging. It autodefaults to any fn
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    event!(Level::INFO, "type: {:#?}", commands.type_rust);
    let exclusions = ai_interactions::return_prompt()?.patchdog_settings.excluded_files;
    let dir = env::current_dir()?;
    let excluded_paths = exclusions
        .par_iter()
        .map(
        |path|dir.join(Path::new(path))
        )
        .collect::<Vec<PathBuf>>();
    let request = changes_from_patch(
        patch, 
        commands.type_rust, 
        commands.name_rust, 
        &excluded_paths
    )?;
    //Here occurs check for pending changes
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

pub async fn call(request: Vec<Request>) -> Result<Vec<ResponseForm>, ErrorBinding> {
    let mut responses_collected = Vec::new();
    let mut pool_of_requests = HashMap::new();
    request.clone().into_iter().for_each(|each| {
        pool_of_requests.insert(each.uuid, each.data);
    });
    let mut new_buffer = RequestToAgent::new()?;
    let batch = new_buffer.prepare_map(request)?;
    let prepared = RequestToAgent::request_manager(batch)?;
    let response = RequestToAgent::send_batches(&prepared).await?;
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

pub fn cherrypick_response(response: &str) -> Result<Vec<RawResponse>, ErrorHandling> {
    let re = Regex::new(REGEX)?;
    let response_cherrypicked = 
    re.captures_iter(response).filter_map(|cap| {
        serde_json::from_str::<RawResponse>(
            cap
            .get(0)?
            .as_str()
        )
        .ok()
    }).collect::<Vec<RawResponse>>();
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

/// Attempts to repair a potentially malformed JSON response by iteratively truncating the input and appending a closing JSON delimiter.
/// This function is designed as a robust fallback to recover valid JSON structures from partial or broken string inputs, particularly useful when dealing with unreliable external API responses.
/// It tries to deserialize the modified string into a `Vec<RawResponse>` and returns the first successful result.
///
/// # Arguments
///
/// * `output` - A `Vec<String>` representing the lines of a potentially incomplete or malformed JSON string.
///
/// # Returns
///
/// A `Result<Vec<RawResponse>, ErrorHandling>` containing the successfully deserialized responses, or an empty vector if no valid structure can be recovered after all attempts.
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
    //This case will only run when there is no valid structures. Returning empty vector will achieve a complete retry
    Ok(vec![])
}

/// Writes generated comments or other code changes into specified files based on structured response data.
/// It sorts the responses by line number in descending order to prevent index shifting issues when inserting multiple changes into the same file.
/// For each response, it reads the target file, inserts the `new_comment` at the designated line range, and then overwrites the file with the updated content.
///
/// # Arguments
///
/// * `response` - A `Vec<ResponseForm>` containing the data to be written, including file paths, line ranges, and the new comments.
///
/// # Returns
///
/// A `Result<(), ErrorHandling>` indicating success or failure of the write operations.
pub fn write_to_file(response: Vec<ResponseForm>) -> Result<(), ErrorHandling> {
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
        let file = fs::read_to_string(&path).context(InvalidIoOperationsSnafu { path: &path })?;
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
