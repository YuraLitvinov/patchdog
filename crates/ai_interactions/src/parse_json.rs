use rust_parsing::error::{
    CouldNotGetLineSnafu, ErrorBinding, ErrorHandling, InvalidIoOperationsSnafu,
};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use snafu::{OptionExt, ResultExt};
use std::ops::Range;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Clone)]
pub struct ChangeFromPatch {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}

//Makes an export structure from patch
//It takes list of files and processes them into objects containing git changes that could be worked with
/// Processes a list of filenames, parses each Rust file, and generates a vector of `ChangeFromPatch` structs.
///
/// # Arguments
///
/// * `filenames`: A vector of `PathBuf`s representing the filenames to process.
///
/// # Returns
///
/// A `Result` containing a vector of `ChangeFromPatch` structs, or an `ErrorHandling` if any error occurred during file parsing or IO operations.
pub fn make_export(filenames: &Vec<PathBuf>) -> Result<Vec<ChangeFromPatch>, ErrorHandling> {
    let mut output_vec: Vec<ChangeFromPatch> = Vec::new();
    let mut vector_of_changed: Vec<Range<usize>> = Vec::new();
    for filename in filenames {
        let path = env::current_dir()
            .context(InvalidIoOperationsSnafu)?
            .join(filename);

        let parsed_file = RustItemParser::parse_rust_file(&path);
        match parsed_file {
            Ok(value) => {
                for each_object in value {
                    let range = each_object.line_start().context(CouldNotGetLineSnafu)?
                        ..each_object.line_end().context(CouldNotGetLineSnafu)?;
                    vector_of_changed.push(range);
                }
                output_vec.push({
                    ChangeFromPatch {
                        filename: path,
                        range: vector_of_changed.to_owned(),
                    }
                });
                vector_of_changed.clear();
            }
            Err(e) => {
                println!("WARNING!\nSKIPPING {e:?} PLEASE REFER TO ERROR LOG");
                continue;
            }
        }
    }
    Ok(output_vec)
}

/// Checks if code objects of specified types and names are present in a given set of files.
///
/// # Arguments
///
/// * `exported_from_file`: A vector of `ChangeFromPatch` structs.
/// * `rust_type`: A vector of strings representing the desired types.
/// * `rust_name`: A vector of strings representing the desired names.
///
/// # Returns
///
/// A `Result` containing a vector of booleans indicating presence, or an `ErrorBinding` if any error occurred.
pub fn justify_presence(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<bool>, ErrorBinding> {
    let mut vecbool: Vec<bool> = Vec::new();
    for each_item in exported_from_file {
        let file = fs::read_to_string(&each_item.filename).context(InvalidIoOperationsSnafu)?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for object in each_item.range {
            //object.start - 1 is a relatively safe operation, as line number never starts with 0
            let item = &vectorized[object.start - 1..object.end];
            let _catch: Vec<String> =
                FileExtractor::push_to_vector(item, "#[derive(Debug)]".to_string(), true)?;
            //Calling at index 0 because parsed_file consists of a single object
            //Does a recursive check, whether the item is still a valid Rust code
            let parsed_file = &RustItemParser::parse_all_rust_items(&item.join("\n"))?[0];
            let obj_type_to_compare = &parsed_file.object_type().context(CouldNotGetLineSnafu)?;
            let obj_name_to_compare = &parsed_file.object_name().context(CouldNotGetLineSnafu)?;
            if rust_type
                .iter()
                .any(|obj_type| obj_type_to_compare == obj_type)
                && rust_name
                    .iter()
                    .any(|obj_name| obj_name_to_compare == obj_name)
            {
                vecbool.push(true) //present
            }
        }
    }
    Ok(vecbool)
}
