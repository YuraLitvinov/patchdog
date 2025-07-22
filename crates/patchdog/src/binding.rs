use crate::binding::rust_parsing::error::CouldNotGetLineSnafu;
use git_parsing::{Git2ErrorHandling, Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing::ObjectRange;
use rust_parsing::error::{InvalidIoOperationsSnafu, InvalidReadFileOperationSnafu};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{FunctionSignature, RustItemParser, RustParser};
use rust_parsing::{self, ErrorHandling};
use snafu::OptionExt;
use snafu::ResultExt;
use std::env;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::mem::size_of_val;

pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: PathBuf,
    pub line: Vec<usize>,
}
#[derive(Debug)]
pub struct Export {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}
#[derive(Debug)]
pub enum ErrorBinding {
    GitParsing(Git2ErrorHandling),
    RustParsing(ErrorHandling),
}

impl From<Git2ErrorHandling> for ErrorBinding {
    fn from(git: Git2ErrorHandling) -> Self {
        ErrorBinding::GitParsing(git)
    }
}

impl From<ErrorHandling> for ErrorBinding {
    fn from(rust: ErrorHandling) -> Self {
        ErrorBinding::RustParsing(rust)
    }
}

pub fn patch_data_argument(path_to_patch: PathBuf) -> Result<Vec<Export>, ErrorBinding> {
    let path = env::current_dir().context(InvalidReadFileOperationSnafu {
        file_path: &path_to_patch,
    })?;
    let patch = get_patch_data(path.join(path_to_patch), path)?;
    Ok(patch)
}

pub fn export_arguments(
    exported_from_file: Vec<Export>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<(), ErrorBinding> {
    let mut fnsig: Vec<FunctionSignature> = Vec::new();
    for each in exported_from_file {
        println!("{:?}", &each.filename);
        let file = fs::read_to_string(&each.filename).context(InvalidIoOperationsSnafu)?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for obj in each.range {
            let item = &vectorized[obj.start - 1..obj.end];
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
                || rust_name
                    .iter()
                    .any(|obj_name| obj_name_to_compare == obj_name)
            {
                let parsed = RustItemParser::rust_function_parser(&item.join("\n"))?;
                fnsig.push(parsed);
                //let parsed = RustItemParser::rust_ast(&item.join("\n"))?;
            }
        }
    }
    println!("{:?}", size_of_val(&fnsig));

    Ok(())
}
/*
Pushes information from a patch into vector that contains lines
at where there are unique changed objects reprensented with range<usize>
and an according path each those ranges that has to be iterated only once
*/
pub fn get_patch_data(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<Export>, ErrorBinding> {
    let export = patch_export_change(path_to_patch, relative_path)?;
    let mut export_difference: Vec<Export> = Vec::new();
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
        export_difference.push(Export {
            range: vector_of_changed.to_owned(),
            filename: difference.filename.to_owned(),
        });
        vector_of_changed.clear();
    }
    Ok(export_difference)
}
//Makes an export structure from files
//It takes list of files and processes them into objects that could be worked with
pub fn make_export(filenames: &Vec<PathBuf>) -> Result<Vec<Export>, ErrorHandling> {
    let mut output_vec: Vec<Export> = Vec::new();
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
                    Export {
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
