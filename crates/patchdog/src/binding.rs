use crate::binding::rust_parsing::error::CouldNotGetLineSnafu;
use ai_interactions::parse_json::ChangeFromPatch;
use gemini::gemini::{ContextData, SingleFunctionData};
use git_parsing::{Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu, InvalidReadFileOperationSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use snafu::{OptionExt, ResultExt};
use std::{
    fs, env, ops::Range, 
    path::{Path, PathBuf}
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

pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<SingleFunctionData>, ErrorBinding> {
    let mut singlerequestdata: Vec<SingleFunctionData> = Vec::new();
    for each in exported_from_file {
        println!("{:?}", &each.filename);
        let file = fs::read_to_string(&each.filename).context(InvalidIoOperationsSnafu)?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for obj in each.range {
            let item = &vectorized[obj.start - 1..obj.end];
            //Calling at index 0 because parsed_file consists of a single object
            //Does a recursive check, whether the item is still a valid Rust code
            let parsed_file = &RustItemParser::rust_item_parser(&item.join("\n"))?;
            let obj_type_to_compare = &parsed_file.object_type().context(CouldNotGetLineSnafu)?;
            let obj_name_to_compare = &parsed_file.object_name().context(CouldNotGetLineSnafu)?;
            if rust_type
                .iter()
                .any(|obj_type| obj_type_to_compare == obj_type)
                || rust_name
                    .iter()
                    .any(|obj_name| obj_name_to_compare == obj_name)
            {
                let as_string = item.join("\n");
            
                singlerequestdata.push(SingleFunctionData {
                    function_text: as_string,
                    fn_name: parsed_file
                        .object_name()
                        .context(CouldNotGetLineSnafu)?,
                    context: ContextData {
                        class_name: "".to_string(),
                        filepath: format!("{:?}", each.filename),
                        external_dependecies: vec![],
                        old_comment: vec!["".to_string()],
                        line_range: obj,
                    }
                });
            }
        }
    }
    Ok(singlerequestdata)
}

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
            let range = each_parsed.line_start().context(CouldNotGetLineSnafu)?
                ..each_parsed.line_end().context(CouldNotGetLineSnafu)?;
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
            let file = fs::read_to_string(&path)
                .context(InvalidReadFileOperationSnafu { file_path: &path })?;
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
