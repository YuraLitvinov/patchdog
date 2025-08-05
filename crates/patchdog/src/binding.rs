use ai_interactions::parse_json::ChangeFromPatch;
use gemini::gemini::{Context, Metadata, Request, SingleFunctionData};
use git_parsing::{Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu, InvalidReadFileOperationSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use snafu::ResultExt;
use std::{
    env, fs,
    ops::Range,
    path::{Path, PathBuf},
};
use tracing::{Level, event};

pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: PathBuf,
    pub line: Vec<usize>,
}

/// Filters and transforms a list of `ChangeFromPatch` structs into `Request` structs, focusing on code changes that match specified Rust object types and names.
/// For each `ChangeFromPatch`, it reads the associated file, extracts the relevant code snippet based on line ranges, parses it, and then creates a `Request` if the object's type or name matches the provided criteria.
///
/// # Arguments
///
/// * `exported_from_file`: A `Vec<ChangeFromPatch>` containing file paths and ranges of changes.
/// * `rust_type`: A `Vec<String>` specifying the desired Rust object types (e.g., "fn", "struct") to filter by.
/// * `rust_name`: A `Vec<String>` specifying the desired Rust object names to filter by.
///
/// # Returns
///
/// A `Result` containing a `Vec<Request>` of filtered code changes, each with a generated UUID, the function text, name, context, and metadata, or an `ErrorBinding` if file I/O or parsing fails.
pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<Request>, ErrorBinding> {
    let mut singlerequestdata: Vec<Request> = Vec::new();
    for each in exported_from_file {
        event!(Level::INFO, "{:?}", &each.filename);
        let file = fs::read_to_string(&each.filename).context(InvalidIoOperationsSnafu)?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for obj in each.range {
            let item = &vectorized[obj.start - 1..obj.end];
            //Calling at index 0 because parsed_file consists of a single object
            //Does a recursive check, whether the item is still a valid Rust code
            let parsed_file = &RustItemParser::rust_item_parser(&item.join("\n"))?;
            let obj_type_to_compare = parsed_file.object_type().unwrap();
            let obj_name_to_compare = parsed_file.object_name().unwrap();
            if rust_type
                .iter()
                .any(|obj_type| &obj_type_to_compare == obj_type)
                || rust_name
                    .iter()
                    .any(|obj_name| &obj_name_to_compare == obj_name)
            {
                let as_string = item.join("\n");
                singlerequestdata.push(Request {
                    uuid: uuid::Uuid::new_v4().to_string(),
                    data: SingleFunctionData {
                        function_text: as_string,
                        fn_name: obj_name_to_compare,
                        context: Context {
                            class_name: "".to_string(),
                            external_dependecies: vec!["".to_string()],
                            old_comment: vec!["".to_string()],
                        },
                        metadata: Metadata {
                            filepath: each.filename.clone(),
                            line_range: obj,
                        },
                    },
                });
            }
        }
    }
    Ok(singlerequestdata)
}

/// Reads and processes a Git patch file to extract relevant code changes.
/// It first determines the current working directory, then calls `get_patch_data` to parse the patch content and identify changes within Rust code objects.
///
/// # Arguments
///
/// * `path_to_patch`: A `PathBuf` pointing to the Git patch file.
///
/// # Returns
///
/// A `Result` containing a `Vec<ChangeFromPatch>` representing the identified code changes, or an `ErrorBinding` if file operations or patch parsing fails.
pub fn patch_data_argument(path_to_patch: PathBuf) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let path = env::current_dir().context(InvalidReadFileOperationSnafu {
        file_path: &path_to_patch,
    })?;

    //let path = Path::new("/home/yurii-sama/embucket").to_path_buf();
    let patch = get_patch_data(path.join(path_to_patch), path)?;
    Ok(patch)
}

/*
Pushes information from a patch into vector that contains lines
at where there are unique changed objects reprensented with range<usize>
and an according path each those ranges that has to be iterated only once
*/
/// Extracts and processes data from a Git patch file, converting it into a vector of `ChangeFromPatch` structs.
/// This involves exporting raw changes from the patch, then parsing the Rust items within those changes to determine which lines fall within identifiable code objects.
///
/// # Arguments
///
/// * `path_to_patch`: A `PathBuf` pointing to the patch file.
/// * `relative_path`: A `PathBuf` indicating the base directory relative to which file paths in the patch are resolved.
///
/// # Returns
///
/// A `Result` containing a `Vec<ChangeFromPatch>` that represents the identified code changes within Rust files, or an `ErrorBinding` if any parsing or I/O error occurs.
pub fn get_patch_data(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let export = patch_export_change(path_to_patch, relative_path)?;
    let mut export_difference: Vec<ChangeFromPatch> = Vec::new();
    let mut vector_of_changed: Vec<Range<usize>> = Vec::new();
    for difference in export {
        let parsed = RustItemParser::parse_rust_file(&difference.filename)?;
        for each_parsed in &parsed {
            let range = each_parsed.line_start()?..each_parsed.line_end()?;
            if difference.line.iter().any(|line| range.contains(line)) {
                vector_of_changed.push(range);
            }
        }
        export_difference.push(ChangeFromPatch {
            range: vector_of_changed.to_owned(),
            filename: difference.filename.to_owned(),
        });
        vector_of_changed.clear();
    }
    Ok(export_difference)
}

/// Stores information about objects affected by a Git patch. It parses the patch to identify changes, and for each file with a single change, it reads the file, parses all Rust items within it, and combines this with the hunk information into `FullDiffInfo` structs.
///
/// # Arguments
///
/// * `relative_path`: A reference to a `Path` representing the base directory for resolving file paths in the patch.
/// * `patch_src`: A byte slice (`&[u8]`) containing the raw content of the Git patch.
///
/// # Returns
///
/// A `Result` containing a `Vec<FullDiffInfo>` that includes the filename, parsed object ranges, and hunk details for each affected file, or an `ErrorBinding` if any file or parsing error occurs.
fn store_objects(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, ErrorBinding> {
    let mut vec_of_surplus: Vec<FullDiffInfo> = Vec::new();
    let matched = match_patch_with_parse(relative_path, patch_src)?;
    for change_line in &matched {
        if change_line.quantity == 1 {
            let list_of_unique_files =
                get_easy_hunk(patch_src, &change_line.change_at_hunk.filename())?;
            let path = relative_path.join(change_line.change_at_hunk.filename());
            let file = fs::read_to_string(&path)?;
            let parsed = RustItemParser::parse_all_rust_items(&file)?;
            vec_of_surplus.push(FullDiffInfo {
                name: change_line.change_at_hunk.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            });
        }
    }

    Ok(vec_of_surplus)
}
/// Exports detailed change information from a Git patch file, focusing on lines that correspond to valid Rust code objects.
/// It first parses the patch to store objects, then for each identified diff hunk, it reads the corresponding file, parses its Rust items, and checks if the changed lines fall within a recognized Rust object.
///
/// # Arguments
///
/// * `path_to_patch`: A `PathBuf` pointing to the patch file.
/// * `relative_path`: A `PathBuf` indicating the base directory for resolving file paths.
///
/// # Returns
///
/// A `Result` containing a `Vec<Difference>` where each `Difference` includes the filename and a list of line numbers that represent changes within valid Rust code objects, or an `ErrorBinding` if any file I/O or parsing error occurs.
fn patch_export_change(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<Difference>, ErrorBinding> {
    let mut change_in_line: Vec<usize> = Vec::new();
    let mut line_and_file: Vec<Difference> = Vec::new();
    let patch_text = fs::read(&path_to_patch).context(InvalidReadFileOperationSnafu {
        file_path: path_to_patch,
    })?;
    let each_diff = store_objects(&relative_path, &patch_text)?;
    for diff_hunk in &each_diff {
        let path_to_file = relative_path.to_owned().join(&diff_hunk.name);
        let file = fs::read_to_string(&path_to_file).context(InvalidIoOperationsSnafu)?;
        let parsed = RustItemParser::parse_all_rust_items(&file)?;
        let path = path_to_file;

        for each in &diff_hunk.hunk {
            let parsed_in_diff = &parsed;
            if FileExtractor::check_for_valid_object(parsed_in_diff, each.get_line())? {
                continue;
            }
            change_in_line.push(each.get_line());
        }
        line_and_file.push(Difference {
            filename: path,
            line: change_in_line.to_owned(),
        });
        change_in_line.clear();
    }
    Ok(line_and_file)
}
