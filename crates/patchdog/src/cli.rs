use clap::CommandFactory;
//Unlike Path, PathBuf size is known at compile time and doesn't require lifetime specifier
#[allow(unused)]
use clap::{Arg, ArgAction, Command, Parser};
use rust_parsing::error::InvalidIoOperationsSnafu;
use snafu::ResultExt;
use std::path::{PathBuf, Path};

use crate::binding::{ErrorBinding, export_arguments, make_export, patch_data_argument};
const EMPTY_VALUE: &str = " ";
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    //patch_path only receives one positional argument containing path to patch
    #[arg(long, default_value = EMPTY_VALUE)]
    patch_path: PathBuf,
    #[arg(long, num_args=1..14, default_value = EMPTY_VALUE)]
    type_rust: Vec<String>,
    #[arg(long, num_args=1.., default_value = EMPTY_VALUE)]
    name_rust: Vec<String>,
    #[arg(long, num_args=1..)]
    rust_path: Vec<PathBuf>,
}

pub fn cli_patch_mode() -> Result<(), ErrorBinding> {
    let commands = Args::parse();
    if !commands.rust_path.is_empty() {
        let file_export = make_export(commands.rust_path)?;
        export_arguments(file_export, commands.type_rust, commands.name_rust)?;
    } else if commands.patch_path == Path::new(EMPTY_VALUE) {
        Args::command().print_help().context(InvalidIoOperationsSnafu)?;
    }
    else  {
        let patch_data = patch_data_argument(commands.patch_path)?;
        export_arguments(patch_data, commands.type_rust, commands.name_rust)?;
    }

    Ok(())
}
