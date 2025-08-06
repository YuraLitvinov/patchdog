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

/// The primary asynchronous function for the CLI application. It parses command-line arguments, processes a specified patch file to identify Rust code changes, sends these changes to an external AI agent for processing, and then writes the agent's responses back to the relevant files.
///
/// # Returns
///
/// An `Ok(())` on successful execution, or an `ErrorBinding` if any step in the process (CLI parsing, patch processing, agent communication, or file writing) fails.
pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    event!(Level::INFO, "type: {:#?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
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
