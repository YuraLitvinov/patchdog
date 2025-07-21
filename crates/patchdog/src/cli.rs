use clap::ArgGroup;
//Unlike Path, PathBuf size is known at compile time and doesn't require lifetime specifier
use crate::binding::{ErrorBinding, export_arguments, make_export, patch_data_argument};
#[allow(unused)]
use clap::{Arg, ArgAction, Command, Parser};

use std::fs;
use std::path::{Path, PathBuf};
const EMPTY_VALUE: &str = " ";
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
    export_arguments(file_export, commands.type_rust, commands.name_rust)?;
    println!("rust files len {}", &rust_files.len());
    Ok(())
}

pub async fn cli_search_patch() -> Result<(), ErrorBinding> {
    let commands = Mode::parse();
    let patch = patch_data_argument(commands.file_patch)?;
    println!("type: {:?}", commands.type_rust);
    export_arguments(patch, commands.type_rust, commands.name_rust)?;
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
