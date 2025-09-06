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

fn is_file_allowed(file: &Path, exclusions: &[PathBuf]) -> Result<bool, ErrorBinding> {
    let starts = exclusions.iter().all(|path| Path::new(path).starts_with(&file));
    if starts == false {
        for path in exclusions.iter() {
            if Path::new(path) == file {
                return Ok(false);
            }
        }
    }
    Ok(!starts)
}

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

/// Processes a Git patch file to extract structured information about code changes, specifically identifying modified objects and their line ranges. It resolves the provided relative patch path, then delegates to `get_patch_data` to parse the patch and convert its contents into a vector of `ChangeFromPatch` structs.
///
/// # Arguments
/// * `path_to_patch` - A `PathBuf` representing the path to the Git patch file, relative to the current working directory.
///
/// # Returns
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>` containing a vector of `ChangeFromPatch` structs, each detailing filenames and ranges of changes, or an `ErrorBinding` if any file system or patch parsing error occurs.
pub fn patch_data_argument(path_to_patch: PathBuf) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let path = env::current_dir()?;
    let patch = get_patch_data(path.join(path_to_patch), path)?;
    Ok(patch)
}

/// Extracts changed code objects from a patch file, identifying their line ranges and filenames. This function first processes the patch to find all differences and then parses the affected Rust files to map these changes to specific `ObjectRange` instances.
/// It uses parallel iteration for efficiency, making it suitable for larger patches or codebases. The output provides a structured view of all significant code alterations, making it easier to pinpoint exact changes.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` pointing to the patch file.
/// * `relative_path` - A `PathBuf` indicating the base directory to resolve file paths within the patch.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>` containing a vector of `ChangeFromPatch` objects, each detailing changed ranges within a file, or an `ErrorBinding` if parsing or file operations fail.
/*
Pushes information from a patch into vector that contains lines
at where there are unique changed objects reprensented with range<usize>
and an according path each those ranges that has to be iterated only once
*/
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

/// Parses a patch file to identify changed lines within Rust code objects and associates them with their respective files. This function reads the patch, extracts diff hunks, and then iterates through relevant Rust files to determine which `ObjectRange` items (e.g., functions, structs) are affected by the changes.
/// It ultimately returns a structured list of `Difference` objects, each containing a filename and a vector of line numbers that have been modified.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` pointing to the patch file to be analyzed.
/// * `relative_path` - A `PathBuf` representing the base directory for resolving file paths mentioned in the patch.
///
/// # Returns
///
/// A `Result<Vec<Difference>, ErrorBinding>` containing a vector of `Difference` objects, each indicating the filename and the lines affected by the patch, or an `ErrorBinding` if any file or parsing operation fails.
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
