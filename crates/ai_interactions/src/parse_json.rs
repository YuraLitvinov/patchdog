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

/// Parses a list of Rust source files to extract information about their contained Rust items.
/// For each file provided, it identifies the line ranges of functions, structs, enums, or other code objects.
/// This information is then compiled into a vector of `ChangeFromPatch` structs, where each entry associates a filename with a list of line ranges corresponding to discovered Rust items.
///
/// # Arguments
///
/// * `filenames` - A reference to a `Vec<PathBuf>` containing the paths to the Rust files to be processed.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorHandling>` containing the extracted file and object range information, or an error if parsing fails for any file.
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

/// Determines the presence of specific Rust items within designated code changes by analyzing their type and name.
/// This function iterates through file modifications described by `exported_from_file`, reads the file content, extracts code segments based on provided line ranges, and then parses these segments into Rust items.
/// For each identified Rust item, it checks if its type name is present in `rust_type` and its item name is present in `rust_name`, pushing `true` to the result vector if both conditions are met.
///
/// # Arguments
/// * `exported_from_file` - A vector of `ChangeFromPatch` objects, detailing file paths and line ranges within them to inspect for Rust items.
/// * `rust_type` - A vector of strings representing the Rust type names to match against extracted items.
/// * `rust_name` - A vector of strings representing the Rust item names (e.g., function names, struct names) to match against extracted items.
///
/// # Returns
/// * `Result<Vec<bool>, ErrorBinding>` - A `Result` containing a vector of booleans, where each `true` indicates that a matching Rust item was found within a processed code range. Returns an `ErrorBinding` if file operations or parsing fail.
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
