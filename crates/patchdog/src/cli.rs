use crate::binding::{self, changes_from_patch};
use clap::ArgGroup;
use clap::Parser;
use gemini::gemini::{GoogleGemini, Response, SingleFunctionData, WaitForTimeout};
use regex::Regex;
use rust_parsing::error::ErrorBinding;
use rust_parsing::error::ErrorHandling;
use rust_parsing::file_parsing::REGEX;
use rust_parsing::file_parsing::{FileExtractor, Files};
use std::collections::HashMap;
use std::{path::PathBuf, fs};
use rust_parsing::error::InvalidIoOperationsSnafu;
use snafu::ResultExt;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, group(
    ArgGroup::new("path")
        .args(["file_patch"])
        .required(true)
)
)]
struct Mode {
    #[arg(long)]
    file_patch: PathBuf,
    #[arg(long, num_args=1..14, requires = "file_patch", default_value = "fn")]
    type_rust: Vec<String>,
    #[arg(long, num_args=1..,  requires = "file_patch")]
    name_rust: Vec<String>,
}
#[derive(Debug)]
struct Form {
    is_full: bool,
    data: SingleFunctionData, 
    new_comment: String
}

/// Processes command-line arguments, extracts code changes, sends them to an AI agent, receives the responses, and writes them to a file.
///
/// # Returns
///
/// A `Result` indicating whether the process was successful, or an `ErrorBinding` if any error occurred.
pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
    println!("Request len: {}", &request.len()); 
    let responses_collected = call(request).await?;
    println!("{}", responses_collected.len());
    write_to_file(responses_collected)?;
    Ok(())
}

/// Collects responses from a string using a regular expression.
///
/// # Arguments
///
/// * `response`: The response string.
///
/// # Returns
///
/// A `Result` containing a vector of `Response` structs, or an `ErrorHandling` if any error occurred.
pub fn collect_response(response: &str) -> Result<Vec<Response>, ErrorHandling> {
    let re = Regex::new(REGEX).unwrap();
    let mut response_from_regex = vec![];
    for cap in re.captures_iter(response) {
        let a = cap.get(0).unwrap().as_str();
        let to_struct = serde_json::from_str::<Response>(a);
        match to_struct {
            Ok(ok) => {
                response_from_regex.push(ok);
            }
            Err(e) => {
                println!("{e}");
                continue;
            }
        }
    }
    Ok(response_from_regex)
}
/// Sends a batch of requests to the Google Gemini API and collects the responses.
///
/// # Arguments
///
/// * `request`: A vector of `SingleFunctionData` structs representing the requests to send.
///
/// # Returns
///
/// A `Result` containing a vector of `Form` structs representing the successful responses, or an `ErrorBinding` if any error occurred.
async fn call(
    request: Vec<SingleFunctionData>,
) -> Result<Vec<Form>, ErrorBinding> {
    let mut responses_collected = Vec::new();
    let mut collect_error = vec![];
    let mut new_buffer = GoogleGemini::new();
    let batch = new_buffer.prepare_map(request)?;
    let prepared = GoogleGemini::assess_batch_readiness(batch)?;
    let response = GoogleGemini::send_batches(&prepared).await?;
    for each in response {
        let matches = match_response(&each, &prepared)?;
        for matched in matches {
            if matched.is_full {
                responses_collected.push(Form{is_full: matched.is_full, data: matched.data, new_comment: matched.new_comment});
            } else {
                collect_error.push(matched.data);
                println!("Found error");
            }
        }
    }
    if !collect_error.is_empty() {
        println!("Found error");
        let collect_error = Box::pin(call(collect_error)).await?;
        responses_collected.extend(collect_error);
    }
    Ok(responses_collected)
}
/// Matches a response string with prepared requests and returns a vector of `Form` structs.
///
/// # Arguments
///
/// * `response`: The response string.
/// * `prepared`: A vector of `WaitForTimeout` structs representing prepared requests.
///
/// # Returns
///
/// A `Result` containing a vector of `Form` structs, or an `ErrorHandling` if any error occurred.
fn match_response(
    response: &str,
    prepared: &Vec<WaitForTimeout>,
) -> Result<Vec<Form>, ErrorHandling> {
    let response_from_regex = collect_response(response)?;
    match serde_json::from_str::<Vec<Response>>(response) {
        Ok(ok) => {
            if response_from_regex.len() == ok.len() {
                let res = match_request_response(prepared, &ok)?;
                return Ok(res);
            }
        }
        Err(_) => {
            let as_vec = FileExtractor::string_to_vector(response);
            let a = &as_vec[1..as_vec.len() - 1];
            let to_struct = fallback_repair(a.to_vec())?;
            if response_from_regex.len() == to_struct.len() {
                let res = match_request_response(prepared, &to_struct)?;
                return Ok(res);
            }
        }
    }
    println!("Failed to match response");
    println!("{}", response);
    Err(ErrorHandling::CouldNotGetLine)
}

//Here we should form a structure, that would consist of request metadata and new comment
/// Matches requests and responses based on UUIDs.
///
/// # Arguments
///
/// * `prepared`: A vector of `WaitForTimeout` structs representing prepared requests.
/// * `uuid`: A vector of `Response` structs representing responses.
///
/// # Returns
///
/// A `Result` containing a vector of `Form` structs, or an `ErrorHandling` if any error occurred.
fn match_request_response(
    prepared: &Vec<WaitForTimeout>,
    uuid: &Vec<Response>,
) -> Result<Vec<Form>, ErrorHandling> {
let mut matched = vec![];
        let mut set = HashMap::new();
        for each in uuid.clone() {
            set.insert(each.uuid, each.new_comment);
        }
        for prepare in prepared {
            for req in &prepare.prepared_requests {
                for each in &req.data {
                    if let Some(found) = set.iter().find(|item| item.0 == each.0) {
                        matched.push(Form { is_full: true, data: each.1.clone(), new_comment: found.1.to_string() });
                    }
                    else {
                        println!("Found error134: {:#?}", each);
                        matched.push(Form { is_full: false, data: each.1.clone(), new_comment: "".to_string() });
                    }
                }
        }
        }
        Ok(matched)
}

fn fallback_repair(output: Vec<String>) -> Result<Vec<Response>, ErrorHandling> {
    let mut clone_out = output;
    for _ in 0..clone_out.len() {
        clone_out.pop();
        let mut clone_clone = clone_out.clone();
        //Fixing broken delimiters in returned JSON here
        clone_clone.push("}]".to_string());
        let _ = match serde_json::from_str::<Vec<Response>>(&clone_clone.join("\n")) {
            Ok(res) => {
                return Ok(res);
            }
            Err(_) => {
                continue;
            }
        };
    }
    println!("Prevent stackoverflow");
    Err(ErrorHandling::CouldNotGetLine)
}

fn write_to_file(response: Vec<Form>) -> Result<(), ErrorHandling> {
    let mut response = response;
    response.sort_by(|a, b| {
        b.data.context
            .line_range
            .start
            .cmp(&a.data.context.line_range.start)
    });

    //Typical representation of file as vector of lines
    for each in response {
        let path = each.data.context.filepath;
        let file =
            fs::read_to_string(&path).context(InvalidIoOperationsSnafu)?;
        let as_vec = FileExtractor::string_to_vector(&file);
        FileExtractor::write_to_vecstring(
            path,
            as_vec,
            each.data.context.line_range.start,
            each.new_comment,
        )?;
    }
    Ok(())
}
