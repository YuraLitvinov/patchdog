use clap::ArgGroup;
use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::ErrorHandling;
use snafu::ResultExt;
//Unlike Path, PathBuf size is known at compile time and doesn't require lifetime specifier
use crate::binding::{changes_from_patch, patch_data_argument};
use ai_interactions::parse_json::make_export;
#[allow(unused)]
use clap::{Arg, ArgAction, Command, Parser};
use gemini::gemini::{GoogleGemini, PreparingRequests, SingleRequestData};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::{BufWriter, Write};
const EMPTY_VALUE: &str = " ";
const _PATH_BASE: &str = "/home/yurii-sama/patchdog/tests/data.rs";
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, group(
    ArgGroup::new("path")
        .args(["dir_path", "file_patch"])
        .required(true)
)
)]
pub struct Mode {
    #[arg(long, short, default_value = EMPTY_VALUE)]
    pub dir_path: PathBuf,
    #[arg(long, default_value = EMPTY_VALUE)]
    file_patch: PathBuf,
    #[arg(long, num_args=1..14, requires = "file_patch", default_value = "fn")]
    type_rust: Vec<String>,
    #[arg(long, num_args=1..,  requires = "file_patch")]
    name_rust: Vec<String>,
}

pub async fn cli_search_mode() -> Result<(), ErrorBinding> {
    let mut rust_files: Vec<PathBuf> = Vec::new();
    let commands = Mode::parse();
    find_rust_files(&commands.dir_path, &mut rust_files);
    let file_export = make_export(&rust_files)?;
    changes_from_patch(file_export, commands.type_rust, commands.name_rust)?;
    println!("rust files len {}", &rust_files.len());
    Ok(())
}

pub async fn cli_patch_to_agent() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    let request = changes_from_patch(patch.clone(), commands.type_rust, commands.name_rust)?;
    let mut new_buffer = GoogleGemini::new();
    let batch = new_buffer.prepare_batches(request.clone())?;
    let prepared = GoogleGemini::assess_batch_readiness(batch.clone()).await?; 
    let response = GoogleGemini::send_batches(&prepared).await?;
    //Attempt to fix broken gemini output
    let fixed = hotfix(response, request)?;
    //Repackaging corrected input. Currently still thinking how to push it to LLM.
    let repack = new_buffer.prepare_batches(fixed.clone())?;
    println!("for LLM: {:#?}", batch);
    println!("repackaged: {:#?}", repack);
    println!("partial return:\n{:#?}", fixed);
    Ok(())
}

pub fn call_json_to_rust(output: Vec<String>) -> Result<PreparingRequests, ErrorHandling> {
    let mut new= vec![];
    let mut clone_out = output.clone();
    for _ in 0..clone_out.len() {
        clone_out.pop();
        let mut clone_clone = clone_out.clone();
        //Fixing broken delimiters in returned JSON here
        clone_clone.push("}]}".to_string());
        let _ = match serde_json::from_str::<PreparingRequests>(&clone_clone.join("\n")) {
            Ok(res) =>  {
                new.push(res);
                continue;
            },
            Err(_) => {
                continue;
            }
        };
    }
    Ok(new.first().unwrap().clone())

}

fn find_rust_files(dir: &Path, rust_files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_rust_files(&path, rust_files); // Recurse into subdirectory
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    rust_files.push(path);
                }
            }
        }
    }
}

fn hotfix(response: Vec<String>, request: Vec<SingleRequestData>)-> Result<Vec<SingleRequestData>, ErrorHandling> {
    let mut partial_return = vec![];
    //Attempt to fix broken gemini output
    for each in response {
        let as_vec = FileExtractor::string_to_vector(&each);
        //Removing the lines, containing ```json``` backticks
        let remove_first_last_line = as_vec[1..as_vec.len().saturating_sub(1)].to_vec();
        let a = call_json_to_rust(remove_first_last_line)?;
        for each in a.data {
            partial_return.push(each);
        }
    }   

    let correct = request[partial_return.len() - 1..request.len() - 1].to_vec();
    Ok(correct)
}

pub fn write_to_file(response: Vec<SingleRequestData>) -> Result<(), ErrorHandling>{
    let mut clone_response = response.clone();
    clone_response.sort_by(|a, b |b.line_range.start.cmp(&a.line_range.start));
    //Typical representation of file as stream of lines
    for each in response {
        let file = File::open(&each.filepath)
            .context(InvalidIoOperationsSnafu)?;
        let mut writer = BufWriter::new(file);
        let mut as_vec = FileExtractor::string_to_vector(&each.filepath);
        as_vec.insert(each.line_range.start, each.comment);
    
    }
    Ok(())
}