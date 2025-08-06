use rust_parsing::error::{ErrorBinding, ErrorHandling};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use std::ops::Range;
use std::path::PathBuf;
use std::{env, fs};
use tracing::{Level, event};

#[derive(Debug, Clone)]
pub struct ChangeFromPatch {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}

//Makes an export structure from patch
//It takes list of files and processes them into objects containing git changes that could be worked with
/// Processes a list of Rust filenames, parses each file to identify code item line ranges, and aggregates this information into a structured format.
/// For each provided file path, the function attempts to read and parse the Rust source code. It then extracts the start and end line numbers for various Rust code items found within the file (e.g., functions, structs, enums).
/// Files that cause parsing errors or non-critical I/O errors during this process will be skipped, and a warning will be logged.
///
/// # Arguments
///
/// * `filenames` - A reference to a `Vec<PathBuf>`, where each `PathBuf` represents the path to a Rust source file to be processed.
///
/// # Returns
///
/// A `Result` which is:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector of `ChangeFromPatch` structs. Each `ChangeFromPatch` contains the path to a processed file and a vector of `Range<usize>` objects, with each range representing the start and end line numbers of a detected code item within that file.
/// - `Err(ErrorHandling)`: An `ErrorHandling` enum variant if a critical I/O error occurs, such as being unable to determine the current directory (`InvalidIoOperationsSnafu`). Individual file parsing errors or read errors result in the file being skipped and a warning logged, not an `Err` return.
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

/// Verifies the presence of specific Rust code objects (filtered by type and name) within the code ranges extracted from a list of files.
/// It reads each file, extracts relevant code segments based on provided ranges, parses them as Rust items, and then checks if their type and name match the desired criteria.
///
/// # Arguments
///
/// * `exported_from_file`: A `Vec<ChangeFromPatch>` representing files and their changed line ranges to check.
/// * `rust_type`: A `Vec<String>` containing the types of Rust objects to look for (e.g., "fn", "struct").
/// * `rust_name`: A `Vec<String>` containing the names of Rust objects to look for.
///
/// # Returns
///
/// A `Result` containing a `Vec<bool>`, where `true` indicates a matching object was found in the corresponding range, or an `ErrorBinding` if any file or parsing error occurs.
pub fn justify_presence(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<bool>, ErrorBinding> {
    let mut vecbool: Vec<bool> = Vec::new();
    for each_item in exported_from_file {
        let file = fs::read_to_string(&each_item.filename)?;
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
