use rust_parsing::error::{ErrorBinding, ErrorHandling, InvalidIoOperationsSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use snafu::ResultExt;
use std::ops::Range;
use std::path::PathBuf;
use std::{env, fs};
use tracing::{Level, event};

#[derive(Debug, Clone)]
pub struct ChangeFromPatch {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}

/// Processes a list of file paths, parses each Rust file to identify changed code ranges,
/// and aggregates these changes into a vector of `ChangeFromPatch` structs.
///
/// # Arguments
///
/// * `filenames` - A reference to a `Vec<PathBuf>` containing the paths to the files to be processed.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorHandling>`:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector where each `ChangeFromPatch` contains the filename and a list of changed line ranges within that file.
/// - `Err(ErrorHandling)`: If any file operation or parsing fails.
pub fn make_export(filenames: &Vec<PathBuf>) -> Result<Vec<ChangeFromPatch>, ErrorHandling> {
    let mut output_vec: Vec<ChangeFromPatch> = Vec::new();
    let mut vector_of_changed: Vec<Range<usize>> = Vec::new();
    for filename in filenames {
        let path = env::current_dir()?
            .join(filename);
        let parsed_file = RustItemParser::parse_rust_file(&path);
        match parsed_file {
            Ok(value) => {
                for each_object in value {
                    let range = each_object.line_ranges.start..each_object.line_ranges.end;
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
                event!(Level::WARN, "{e:#?}");
                continue;
            }
        }
    }
    Ok(output_vec)
}

pub fn justify_presence(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<bool>, ErrorBinding> {
    let mut vecbool: Vec<bool> = Vec::new();
    for each_item in exported_from_file {
        let file = fs::read_to_string(&each_item.filename)
            .context(InvalidIoOperationsSnafu { path: each_item.filename })?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for object in each_item.range {
            //object.start - 1 is a relatively safe operation, as line number never starts with 0
            let item = &vectorized[object.start - 1..object.end];
            let _catch: Vec<String> =
                FileExtractor::push_to_vector(item, "#[derive(Debug)]".to_string(), true)?;
            //Calling at index 0 because parsed_file consists of a single object
            //Does a recursive check, whether the item is still a valid Rust code
            let parsed_file = &RustItemParser::parse_all_rust_items(&item.join("\n"))?[0];
            let obj_type_to_compare = &parsed_file.names.type_name;
            let obj_name_to_compare = &parsed_file.names.name;
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
