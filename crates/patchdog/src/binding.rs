use ai_interactions::parse_json::ChangeFromPatch;
use gemini::gemini::{Context, Metadata, Request, SingleFunctionData};
use git_parsing::{Hunk, get_easy_hunk, match_patch_with_parse};
use git2::Diff;
use rayon::prelude::*;
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::error::ErrorBinding;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use std::{
    env, fs,
    ops::Range,
    path::{Path, PathBuf},
};

pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: PathBuf,
    pub line: Vec<usize>,
}

struct LocalChange {
    filename: PathBuf,
    range: Range<usize>,
    file: String,
}

/// Transforms a collection of file changes (`ChangeFromPatch`) into a vector of `Request` objects.
/// It reads the content of each affected file, extracts specific code snippets based on line ranges,
/// parses these snippets to identify Rust functions or items, and then filters them based on `rust_type` or `rust_name`.
/// Each matching item is then wrapped into a `Request` struct with a unique UUID, function data, and metadata.
///
/// # Arguments
///
/// * `exported_from_file` - A `Vec<ChangeFromPatch>` containing file paths and ranges of changes.
/// * `rust_type` - A `Vec<String>` of desired Rust item types to filter for.
/// * `rust_name` - A `Vec<String>` of desired Rust item names to filter for.
///
/// # Returns
///
/// A `Result<Vec<Request>, ErrorBinding>`:
/// - `Ok(Vec<Request>)`: A vector of `Request` objects representing the filtered and transformed code changes.
/// - `Err(ErrorBinding)`: If file reading, string manipulation, or Rust item parsing fails.
pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<Request>, ErrorBinding> {
    let tasks: Vec<LocalChange> = exported_from_file
        .par_iter()
        .flat_map(|each| {
            each.range.par_iter().map(move |obj| LocalChange {
                filename: each.filename.clone(),
                range: obj.clone(),
                file: fs::read_to_string(&each.filename).unwrap(),
            })
        })
        .collect();
    let singlerequestdata: Vec<Request> = tasks
        .par_iter()
        .filter_map(|each| {
            let vectorized = FileExtractor::string_to_vector(&each.file);
            let item = &vectorized[each.range.start - 1..each.range.end];
            let parsed_file = RustItemParser::rust_item_parser(&item.join("\n")).ok()?;
            let obj_type_to_compare = parsed_file.names.type_name;
            let obj_name_to_compare = parsed_file.names.name;
            if rust_type.iter().any(|t| &obj_type_to_compare == t)
                || rust_name.iter().any(|n| &obj_name_to_compare == n)
            {
                let as_string = item.join("\n");
                Some(Request {
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
                            line_range: each.range.clone(),
                        },
                    },
                })
            } else {
                None
            }
        })
        .collect();
    Ok(singlerequestdata)
}

/// Retrieves parsed patch data from a specified patch file.
/// It constructs the absolute path to the patch file and the relative working directory,
/// then calls `get_patch_data` to process the patch.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` indicating the path to the patch file.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>`:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector containing details of changes extracted from the patch.
/// - `Err(ErrorBinding)`: If the current directory cannot be determined or patch data extraction fails.
pub fn patch_data_argument(path_to_patch: PathBuf) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let path = env::current_dir()?;
    let patch = get_patch_data(path.join(path_to_patch), path)?;
    Ok(patch)
}

/*
Pushes information from a patch into vector that contains lines
at where there are unique changed objects reprensented with range<usize>
and an according path each those ranges that has to be iterated only once
*/
/// Processes a Git patch to identify specific code changes within Rust files.
/// It first exports the raw changes from the patch, then iterates through these differences.
/// For each difference, it parses the corresponding Rust file and identifies actual Rust items (functions, structs, etc.)
/// whose line ranges overlap with the reported changes in the patch.
/// The result is a refined list of `ChangeFromPatch` objects containing only relevant Rust item ranges.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` indicating the path to the patch file.
/// * `relative_path` - A `PathBuf` representing the relative base path from which files are referenced.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>`:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector of `ChangeFromPatch` objects, each containing a filename and a vector of `Range<usize>` indicating the line ranges of identified Rust items that were changed by the patch.
/// - `Err(ErrorBinding)`: If patch export fails or Rust file parsing encounters an error.
pub fn get_patch_data(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let export = patch_export_change(path_to_patch, relative_path)?;
    let export_difference = export
        .par_iter()
        .flat_map(|difference| {
            let parsed = RustItemParser::parse_rust_file(&difference.filename).ok()?;
            let vector_of_changed = parsed
                .par_iter()
                .flat_map(|each_parsed| {
                    let range = each_parsed.line_start()..each_parsed.line_end();
                    if difference.line.par_iter().any(|line| range.contains(line)) {
                        Some(range)
                    } else {
                        None
                    }
                })
                .collect();
            Some(ChangeFromPatch {
                range: vector_of_changed,
                filename: difference.filename.to_owned(),
            })
        })
        .collect();
    Ok(export_difference)
}

/// Stores detailed information about changed objects in a Git patch.
/// It parses the raw patch content, matches patch hunks with parsed Rust items, and for each changed file,
/// it extracts relevant hunks and parses the file's Rust items to provide a comprehensive `FullDiffInfo`.
///
/// # Arguments
///
/// * `relative_path` - A reference to a `Path` indicating the base directory for relative file paths.
/// * `patch_src` - A byte slice (`&[u8]`) containing the raw content of the patch file.
///
/// # Returns
///
/// A `Result<Vec<FullDiffInfo>, ErrorBinding>`:
/// - `Ok(Vec<FullDiffInfo>)`: A vector of `FullDiffInfo` structs, each containing the filename, parsed object ranges within that file, and associated hunks.
/// - `Err(ErrorBinding)`: If diff parsing, hunk matching, file reading, or Rust item parsing fails.
fn store_objects(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, ErrorBinding> {
    let diff = Diff::from_buffer(patch_src).unwrap();
    let changes = &match_patch_with_parse(relative_path, &diff)?;
    let vec_of_surplus = changes
        .iter()
        .map(|change| {
            let list_of_unique_files = get_easy_hunk(&diff, &change.filename()).unwrap();
            let path = relative_path.join(change.filename());
            let file = fs::read_to_string(&path).unwrap();
            let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
            FullDiffInfo {
                name: change.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            }
        })
        .collect();
    Ok(vec_of_surplus)
}

/// Exports detailed line differences from a Git patch, focusing on identifying lines that introduce changes within Rust code objects.
/// It reads the patch file, extracts general diff information, then iterates through each changed hunk.
/// For each hunk, it reads the corresponding file, parses its Rust items, and checks if the changed lines within the hunk fall outside of existing, valid Rust objects (indicating new or significantly altered structures).
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` specifying the path to the patch file.
/// * `relative_path` - A `PathBuf` specifying the base directory for relative file paths.
///
/// # Returns
///
/// A `Result<Vec<Difference>, ErrorBinding>`:
/// - `Ok(Vec<Difference>)`: A vector of `Difference` structs, each containing the filename and a list of line numbers that represent changes not neatly contained within existing Rust objects.
/// - `Err(ErrorBinding)`: If file reading, object storage, or Rust item parsing fails.
fn patch_export_change(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<Difference>, ErrorBinding> {
    let mut change_in_line: Vec<usize> = Vec::new();
    let mut line_and_file: Vec<Difference> = Vec::new();
    let patch_text = fs::read(&path_to_patch)?;
    let each_diff = store_objects(&relative_path, &patch_text)?;
    for diff_hunk in &each_diff {
        let path_to_file = relative_path.to_owned().join(&diff_hunk.name);
        let file = fs::read_to_string(&path_to_file)?;
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
