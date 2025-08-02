use crate::binding::{self, changes_from_patch};
use clap::ArgGroup;
use clap::Parser;
use gemini::gemini::{GoogleGemini, Response, SingleFunctionData, WaitForTimeout};
use regex::Regex;
use rust_parsing::error::ErrorBinding;
use rust_parsing::error::ErrorHandling;
use rust_parsing::error::InvalidIoOperationsSnafu;
use rust_parsing::file_parsing::REGEX;
use rust_parsing::file_parsing::{FileExtractor, Files};
use snafu::ResultExt;
use std::fs;
use std::path::PathBuf;
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

pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
    println!("Request len: {}", &request.len()); 
    let responses_collected = call(request).await?;
    println!("{:#?}", &responses_collected);
    write_to_file(responses_collected)?;
    Ok(())
}

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
async fn call(
    request: Vec<SingleFunctionData>,
) -> Result<Vec<(SingleFunctionData, String)>, ErrorBinding> {
    let mut responses_collected = Vec::new();
    let mut collect_error = vec![];
    let mut new_buffer = GoogleGemini::new();
    let batch = new_buffer.prepare_map(request)?;
    let prepared = GoogleGemini::assess_batch_readiness(batch)?;
    let response = GoogleGemini::send_batches(&prepared).await?;
    for each in response {
        let matches = match_response(&each, &prepared)?;
        for matched in matches {
            if matched.0 {
                responses_collected.push((matched.1, matched.2));
            } else {
                collect_error.push(matched.1);
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
fn match_response(
    response: &str,
    prepared: &Vec<WaitForTimeout>,
) -> Result<Vec<(bool, SingleFunctionData, String)>, ErrorHandling> {
    let response_from_regex = collect_response(response)?;
    match serde_json::from_str::<Vec<Response>>(response) {
        Ok(ok) => {
            if response_from_regex.len() == ok.len() {
                if let Some(each) = response_from_regex.first() {
                    let res = match_request_response(prepared, each)?;
                    return Ok(res);
                }
            }
        }
        Err(_) => {
            let as_vec = FileExtractor::string_to_vector(response);
            let a = &as_vec[1..as_vec.len() - 1];
            let to_struct = fallback_repair(a.to_vec())?;
            if response_from_regex.len() == to_struct.len() {
                if let Some(each) = response_from_regex.first() {
                    let res = match_request_response(prepared, each)?;
                    return Ok(res);
                }
            }
        }
    }
    println!("Failed to match response");
    Err(ErrorHandling::CouldNotGetLine)
}
//Here we should form a structure, that would consist of request metadata and new comment
fn match_request_response(
    prepared: &Vec<WaitForTimeout>,
    uuid: &Response,
) -> Result<Vec<(bool, SingleFunctionData, String)>, ErrorHandling> {
    let mut matched: Vec<(bool, SingleFunctionData, String)> = Vec::new();
    for each in prepared {
        for request in &each.prepared_requests {
            let contains = request.data.contains_key(&uuid.uuid);
            for val in &request.data {
                matched.push((contains, val.1.clone(), uuid.new_comment.clone()));
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
    //Error prevents stack overflow
    println!("Prevent stackoverflow");
    Err(ErrorHandling::CouldNotGetLine)
}

fn write_to_file(response: Vec<(SingleFunctionData, String)>) -> Result<(), ErrorHandling> {
    let mut response = response;
    response.sort_by(|a, b| {
        b.0.context
            .line_range
            .start
            .cmp(&a.0.context.line_range.start)
    });
    //Typical representation of file as vector of lines
    for each in response {
        let path = each.0.context.filepath;
        let file =
            fs::read_to_string(&path).context(InvalidIoOperationsSnafu)?;
        let as_vec = FileExtractor::string_to_vector(&file);
        FileExtractor::write_to_vecstring(
            path,
            as_vec,
            each.0.context.line_range.start,
            each.1,
        )?;
    }
    Ok(())
}
