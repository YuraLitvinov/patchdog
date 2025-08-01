use clap::ArgGroup;
use rust_parsing::error::ErrorBinding;
use rust_parsing::rust_parser::{RustItemParser, RustParser};
//Unlike Path, PathBuf size is known at compile time and doesn't require lifetime specifier
use crate::binding::{self, changes_from_patch};
use ai_interactions::parse_json::make_export;
#[allow(unused)]
use clap::{Arg, ArgAction, Parser};
use gemini::gemini::{PreparingRequests, SingleFunctionData, GoogleGemini};
use std::path::{Path, PathBuf};
use std::fs::{File, self};
use rust_parsing::error::ErrorHandling;
use snafu::ResultExt;
use rust_parsing::error::InvalidIoOperationsSnafu;
use rust_parsing::file_parsing::{FileExtractor, Files};
use gemini::gemini::collect_response;
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
    dir_path: PathBuf,
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
    //Error cases handled in this vector
    //let mut responses_collected = Vec::new();
    let commands = Mode::parse();
    let patch = binding::patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
    println!("{}", request.len());     
    let mut new_buffer = GoogleGemini::new();
    let batch = new_buffer.prepare_map(request)?;
    let prepared = GoogleGemini::assess_batch_readiness(batch).await?; 
    let response = GoogleGemini::send_batches(&prepared).await?;
    //Attempt to fix broken gemini output
    for each in response {
        //Assessing the type of return. Whether the return contains backticks, or is a valid PreparingRequests
        //Ok root follows the 'happy path' and checks whether the output matches input 
        //Err variant considers broken JSON structure
        /* 
        match serde_json::from_str::<PreparingRequests>(&each) {
            Ok(ok) => {
                responses_collected = ok_path(ok, &each)?;
            },
            Err(_) => {
                responses_collected = err_path(&each)?;
            }
        }
        */
        println!("{}", each);
    }
    Ok(())
}

fn err_path(source : &str) -> Result<Vec<SingleFunctionData>, ErrorHandling>{
    let collected = collect_response(source)?;
    Ok(collected)
}

fn ok_path (prepared: PreparingRequests, single_response: &str) -> Result<Vec<SingleFunctionData>, ErrorHandling>{ 
    let mut responses_collected = Vec::new();
    for data in prepared.data {
        //Attempt to parse a function - assessing whether it's still valid
        let parsed_result = RustItemParser::rust_function_parser(&data.function_text);
        if parsed_result.is_err() {
            //responses_collected.push(each.function_text);
            responses_collected = err_path(single_response)?;

        }
        else {                   
            //Now, we attempt to locate this function at given path
            let file = fs::read_to_string(&data.context.filepath)
                .context(InvalidIoOperationsSnafu)?;
            //let parsed = RustItemParser::parse_all_rust_items(&file);
            let file_as_vector = FileExtractor::string_to_vector(&file);
            let parsed_original = RustItemParser::rust_function_parser(
                &file_as_vector[data.context.line_range.clone()].join("\n")
            );
            if parsed_original.is_err() {
                //Not good!
               responses_collected = err_path(&single_response)?;

            }
            else {
                if parsed_original? == parsed_result? {
                    //Good! 
                    continue;  
                }
                else {
                   responses_collected = err_path(&single_response)?;

                }
            }
        }
    }
    Ok(responses_collected)
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

pub fn write_to_file(response: Vec<SingleFunctionData>) -> Result<(), ErrorHandling>{
    let mut response = response;
    response.sort_by(|a, b |b.context.line_range.start.cmp(&a.context.line_range.start));
    //Typical representation of file as vector of lines
    for each in response {
        let _file = File::open(&each.context.filepath)
            .context(InvalidIoOperationsSnafu)?;
        let mut as_vec = FileExtractor::string_to_vector(&each.context.filepath);
        as_vec.insert(each.context.line_range.start, each.context.old_comment.join("\n"));
    }
    Ok(())
}