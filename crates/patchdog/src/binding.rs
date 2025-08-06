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
