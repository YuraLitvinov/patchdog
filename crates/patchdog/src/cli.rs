use clap::ArgGroup;
use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu, SerdeSnafu};
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
use std::collections::HashMap;
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
    for each in batch {
        let a = serde_json::to_string_pretty(&each).context(SerdeSnafu)?;
        println!("{}", a);
    }
    /* 
    let response = GoogleGemini::send_batches(&prepared).await?;
    //Attempt to fix broken gemini output
    let fixed = hotfix(response, request)?;
    //Repackaging corrected input. Currently still thinking how to push it to LLM.
    let repack = new_buffer.prepare_batches(fixed.clone())?;
    println!("for LLM: {:#?}", batch);
    println!("repackaged: {:#?}", repack);
    println!("partial return:\n{:#?}", fixed);
    */
    Ok(())
}
//Accepts JSON as Vec<String> and attempts to parse it into PreparingRequests
pub fn call_json_to_rust(output: Vec<String>) -> Result<PreparingRequests, ErrorHandling> {
    let mut clone_out = output.clone();
    for _ in 0..output.len() {
        clone_out.pop();
        let mut clone_clone = clone_out.clone();
        //Fixing broken delimiters in returned JSON here
        clone_clone.push("}]}".to_string());
        let _ = match serde_json::from_str::<PreparingRequests>(&clone_clone.join("\n")) {
            Ok(res) =>  {
                return Ok(res);
            },
            Err(e) => {
                return Err(ErrorHandling::SerdeError { source: e });
            }
        };
    }
    Err(ErrorHandling::CouldNotGetLine)

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
//Hotfix returns missing elements of request that were dropped in the response by the LLM
pub fn hotfix(response: String, request: Vec<SingleRequestData>)-> Result<Vec<SingleRequestData>, ErrorHandling> {
    let mut hotfixed = vec![];
    let as_vec = FileExtractor::string_to_vector(&response);
    let as_req: Vec<SingleRequestData> = call_json_to_rust(as_vec)?.data;
    let mut map_request  = HashMap::new();
    request.clone().into_iter().for_each(|each| {
        map_request.insert((each.clone().filepath, each.clone().line_range), each.clone());
    });
    let mut map_response = HashMap::new();
    as_req.clone().into_iter().for_each(|each| {
        map_response.insert((each.clone().filepath, each.clone().line_range), each.clone());
    });
    //Key here represents filepath and lineranges of an object, i.e. function
    for (key, _) in &map_request {
        if !map_response.contains_key(&key) {
            hotfixed.push(map_request.get(&key).unwrap().clone());
        }
    }
    Ok(hotfixed)
}

pub fn write_to_file(response: Vec<SingleRequestData>) -> Result<(), ErrorHandling>{
    let mut clone_response = response.clone();
    clone_response.sort_by(|a, b |b.line_range.start.cmp(&a.line_range.start));
    //Typical representation of file as stream of lines
    for each in response {
        let _file = File::open(&each.filepath)
            .context(InvalidIoOperationsSnafu)?;
        //let mut writer = BufWriter::new(file);
        let mut as_vec = FileExtractor::string_to_vector(&each.filepath);
        as_vec.insert(each.line_range.start, each.comment);
    
    }
    Ok(())
}