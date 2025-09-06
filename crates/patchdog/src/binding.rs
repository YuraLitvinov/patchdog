use ai_interactions::return_prompt;
use clap::error::Result;
use gemini::request_preparation::{Context, Metadata, Request, SingleFunctionData};
use git_parsing::{Git2ErrorHandling, Hunk, get_easy_hunk, match_patch_with_parse};
use git2::Diff;
use rayon::prelude::*;
use rust_parsing::ObjectRange;
use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use rust_parsing::{self};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::{
    env, fs,
    ops::Range,
    path::{Path, PathBuf},
};

use crate::analyzer::{AnalyzerData, contextualizer};

#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    pub ident: String,
    pub module: String,
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathObject {
    pub filename: PathBuf,
    pub object: ObjectRange,
}

pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: PathBuf,
    pub line: Vec<usize>,
}

pub struct LocalChange {
    pub filename: PathBuf,
    pub range: Range<usize>,
    pub file: String,
}
#[derive(Debug, Clone)]
pub struct LocalContext {
    pub context_type: String,
    pub context_name: String,
    pub context_path: String,
}

#[derive(Debug)]
pub struct ChangeFromPatch {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}

/// Determines if a given `file` path is located within a specified `dir` path.
/// It first canonicalizes both paths to handle symbolic links and relative path components accurately.
/// Returns `Ok(true)` if the canonicalized file path starts with the canonicalized directory path, indicating it belongs to that directory, `Ok(false)` otherwise, or `Err(std::io::Error)` if canonicalization fails.
fn file_belongs_to_dir(file: &Path, dir: &Path) -> std::io::Result<bool> {
    let file_path = fs::canonicalize(file)?;
    let dir_path = fs::canonicalize(dir)?;
    Ok(file_path.starts_with(&dir_path))
}

/// Checks if a given file is allowed for processing, based on a list of exclusion directories.
/// It iterates through the provided `exclusions` list and uses `file_belongs_to_dir` to determine if the `file`'s path falls under any of these excluded directories.
/// This function is crucial for filtering out files that should not be analyzed or modified.
///
/// # Arguments
///
/// * `file` - A reference to a `Path` representing the file to check.
/// * `exclusions` - A slice of `PathBuf` representing directories that are excluded.
///
/// # Returns
///
/// A `std::io::Result<bool>` which is `true` if the file is allowed (not in any excluded directory), and `false` otherwise.
fn is_file_allowed(file: &Path, exclusions: &[PathBuf]) -> std::io::Result<bool> {
    for dir in exclusions {
        if file_belongs_to_dir(file, dir)? {
            return Ok(false);
        }
    }
    Ok(true) // not in any excluded dir
}

/// Processes a collection of `ChangeFromPatch` items to generate structured `Request` objects, typically for an LLM or similar external service.
/// It filters changes based on file exclusion lists and whether the changed Rust item matches specified types or names, also considering functions explicitly excluded in configuration.
/// For each relevant change, the function extracts the full function text and gathers its surrounding context, including external dependencies, to form a comprehensive `Request` that can be sent for further analysis or generation.
/// Processes a collection of `ChangeFromPatch` items to generate structured `Request` objects, typically for an LLM or similar external service.
/// It filters changes based on file exclusion lists and whether the changed Rust item matches specified types or names, also considering functions explicitly excluded in configuration.
/// For each relevant change, the function extracts the full function text and gathers its surrounding context, including external dependencies, to form a comprehensive `Request` that can be sent for further analysis or generation.
///
/// # Arguments
///
/// * `exported_from_file` - A `Vec<ChangeFromPatch>` containing information about code changes.
/// * `rust_type` - A `Vec<String>` of Rust item types to include.
/// * `rust_name` - A `Vec<String>` of Rust item names to include.
/// * `file_exclude` - A slice of `PathBuf` representing files or directories to exclude from processing.
///
/// # Returns
///
/// A `Result<Vec<Request>, ErrorBinding>` containing the processed requests, or an error if context gathering or parsing fails.
pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
    file_exclude: &[PathBuf],
    analyzer_data: AnalyzerData,
) -> Result<Vec<Request>, ErrorBinding> {
    let tasks: Vec<LocalChange> = exported_from_file
        .par_iter()
        .flat_map(|each| {
            each.range.par_iter().filter_map(move |obj| {
                Some(LocalChange {
                    filename: each.filename.clone(),
                    range: obj.clone(),
                    file: fs::read_to_string(&each.filename)
                        .context(InvalidIoOperationsSnafu {
                            path: each.filename.clone(),
                        })
                        .ok()?,
                })
            })
        })
        .collect();
    let singlerequestdata: Vec<Request> = tasks
        .iter()
        .filter_map(|change| {
            //Here we only allow files, that are not in the config.yaml-Patchdog_settings-excluded_files
            if is_file_allowed(&change.filename, file_exclude).ok()? {
                let vectorized = FileExtractor::string_to_vector(&change.file);
                let item = &vectorized[change.range.start - 1..change.range.end];
                let parsed_file = RustItemParser::rust_item_parser(&item.join("\n")).ok()?;
                let obj_type_to_compare = parsed_file.names.type_name;
                let obj_name_to_compare = parsed_file.names.name;
                if rust_type.par_iter().any(|t| &obj_type_to_compare == t)
                    || rust_name.par_iter().any(|n| &obj_name_to_compare == n)
                        && return_prompt()
                            .ok()?
                            .patchdog_settings
                            .excluded_functions
                            .contains(&obj_name_to_compare)
                {
                    //At this point in parsed_file we are already aware of all the referenced data
                    let fn_as_string = item.join("\n");
                    /*
                    Calling find_context(all methods: bla-bla, function: String) -> context(Vec<String>) {
                        1.
                        2. Find matches in code
                        3. Return matching structures
                    }
                    */
                    let k = fs::read_to_string(&change.filename).ok()?;
                    let parse_analyzer = &RustItemParser::parse_result_items(&k).ok()?;
                    let mut lineranges = vec![];
                    for each in parse_analyzer {
                        let changed_syn = change.range.start..change.range.end;
                        let linerange =
                            RustItemParser::textrange_into_linerange(each.0.to_owned(), &k);
                        if linerange == changed_syn {
                            lineranges.push(each.0);
                        }
                    }
                    let analyzer_context = contextualizer(
                        &change.filename,
                        lineranges.first().copied(),
                        &analyzer_data,
                    )
                    .par_iter()
                    .map(|(_, value)| value.to_string())
                    .collect::<Vec<String>>();
                    let context = Context {
                        class_name: "".to_string(),
                        external_dependencies: analyzer_context,
                        old_comment: vec![],
                    };
                    Some(Request {
                        uuid: uuid::Uuid::new_v4().to_string(),
                        data: SingleFunctionData {
                            function_text: fn_as_string,
                            fn_name: obj_name_to_compare,
                            context,
                            metadata: Metadata {
                                filepath: change.filename.clone(),
                                line_range: change.range.clone(),
                            },
                        },
                    })
                } else {
                    None
                }
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
/// Returns a `Result<Vec<ChangeFromPatch>, ErrorBinding>`: `Ok(Vec<ChangeFromPatch>)` with details of changes from the patch, or `Err(ErrorBinding)` if the current directory cannot be determined or patch data extraction fails.
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

/// Parses a Git patch to extract detailed information about changes to Rust source files.
/// For each file identified in the patch, it reads the file's content, parses all Rust items (functions, structs, comments, etc.) within it, and retrieves the specific hunks (changed blocks of code) from the patch.
/// The collected data is then structured into `FullDiffInfo` objects, providing a comprehensive view of the affected files, their parsed Rust items, and the exact lines changed in the patch.
/// Parses a Git patch to extract detailed information about changes to Rust source files.
/// For each file identified in the patch, it reads the file's content, parses all Rust items (functions, structs, comments, etc.) within it, and retrieves the specific hunks (changed blocks of code) from the patch.
/// The collected data is then structured into `FullDiffInfo` objects, providing a comprehensive view of the affected files, their parsed Rust items, and the exact lines changed in the patch.
///
/// # Arguments
///
/// * `relative_path` - A reference to a `Path` representing the base directory relative to which file paths in the patch are resolved.
/// * `patch_src` - A byte slice (`&[u8]`) containing the raw Git patch content.
///
/// # Returns
///
/// A `Result<Vec<FullDiffInfo>, Git2ErrorHandling>` containing detailed information about the changes for each file in the patch, or an error if parsing or file operations fail.
fn store_objects(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, Git2ErrorHandling> {
    let diff = Diff::from_buffer(patch_src)?;
    let changes = &match_patch_with_parse(relative_path, &diff)?;
    let vec_of_surplus = changes
        .iter()
        .filter_map(|change| {
            let list_of_unique_files = get_easy_hunk(&diff, &change.filename()).ok()?;
            let path = relative_path.join(change.filename());
            let file = fs::read_to_string(&path)
                .context(InvalidIoOperationsSnafu { path })
                .ok()?;
            let parsed = RustItemParser::parse_all_rust_items(&file).ok()?;
            Some(FullDiffInfo {
                name: change.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            })
        })
        .collect();
    Ok(vec_of_surplus)
}

/// Processes a Git patch to identify lines of code that have been changed and are *not* part of existing, recognized Rust items.
/// It reads the patch and gathers `FullDiffInfo` for each changed file, then iterates through each hunk to determine which changed lines fall outside of parsed Rust structures.
/// The function returns a list of `Difference` structs, indicating files and specific lines within them that represent new or modified code segments not belonging to an existing function, struct, or other parsed item.
/// Processes a Git patch to identify lines of code that have been changed and are *not* part of existing, recognized Rust items.
/// It reads the patch and gathers `FullDiffInfo` for each changed file, then iterates through each hunk to determine which changed lines fall outside of parsed Rust structures.
/// The function returns a list of `Difference` structs, indicating files and specific lines within them that represent new or modified code segments not belonging to an existing function, struct, or other parsed item.
///
/// # Arguments
///
/// * `path_to_patch` - The `PathBuf` to the Git patch file.
/// * `relative_path` - The `PathBuf` to the root of the repository or project, used to resolve file paths from the patch.
///
/// # Returns
///
/// A `Result<Vec<Difference>, ErrorBinding>` containing a list of files and their associated new/modified lines, or an error if file operations or parsing fail.
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
        let file = fs::read_to_string(&path_to_file).context(InvalidIoOperationsSnafu {
            path: &path_to_file,
        })?;
        let parsed = RustItemParser::parse_result_items(&file)?
            .par_iter()
            .map(|val| {
                let range = RustItemParser::textrange_into_linerange(*val.0, &file);
                ObjectRange {
                    line_ranges: range,
                    names: val.1.names.clone(),
                }
            })
            .collect::<Vec<ObjectRange>>();
        for each in &diff_hunk.hunk {
            let parsed_in_diff = &parsed;
            if FileExtractor::check_for_valid_object(parsed_in_diff, each.get_line())? {
                continue;
            }
            change_in_line.push(each.get_line());
        }
        line_and_file.push(Difference {
            filename: path_to_file,
            line: change_in_line.to_owned(),
        });
        change_in_line.clear();
    }
    Ok(line_and_file)
}
