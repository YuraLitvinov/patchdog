use clap::ArgGroup;
//Unlike Path, PathBuf size is known at compile time and doesn't require lifetime specifier
use crate::binding::{changes_from_patch, patch_data_argument};
use ai_interactions::parse_json::make_export;
use rust_parsing::error::ErrorBinding;
#[allow(unused)]
use clap::Parser;
use gemini::gemini::{GoogleGemini, REQUESTS_PER_MIN};
use std::fs;
use std::path::{Path, PathBuf};
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

pub async fn cli_patch_to_agent(cut_exceeding_batch: bool) -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    let request = changes_from_patch(patch, commands.type_rust, commands.name_rust)?;
    let mut new_buffer = GoogleGemini::new();
    println!("{}", &request.len());
    let batch = new_buffer.prepare_batches(request);
    if batch.len() > REQUESTS_PER_MIN {
        println!("REQUEST HANDLE EXCEEDING REQUEST PER MINUTE COUNT");
        //We should wait 1 min for response before sending next batch
        //Use sleep()
    }
    println!("{:#?}", batch);
    GoogleGemini::send_batch(batch).await;

    //Attempt to form JSON from functions
    //let mut functions: Vec<FnDataEntry> = Vec::new();
    //let mut fn_body: Vec<String> = Vec::new();
    /*
    for each in &new_buffer.preparing_requests.data {
        let foo = &each.function_text;
        let information = FnDataEntry {
        generic_information: RustItemParser::rust_item_parser(&foo).unwrap(),
        fn_top_block: RustItemParser::rust_function_parser(&foo).unwrap(),
        comment: String::new(),
        };
        fn_body.push(foo.to_string());
        functions.push(information);
    }

    println!("{:#?}", functions[0]);
    let file: FileFn = FileFn { filename: "placeholder".to_string(), types: functions };
    let json = json!(file);
    let file_as_json = json!(file).to_string() +
        "\nThe provided data aside from JSON is valid Rust code. Instruction: Locate each function with it's
        correspondent in JSON, generate /// comment for it and fill it in the types-comment block.
        Return same JSON structure with filled in comment block for each function. Dismiss.";
    let response= GoogleGemini::req_res(fn_body.join(""), file_as_json).await?;
    println!("{:#?}", response);
    */

    Ok(())
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
