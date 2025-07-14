use crate::binding::rust_parsing::error::CouldNotGetLineSnafu;
use git_parsing::{Git2ErrorHandling, Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing::ObjectRange;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use rust_parsing::{self, ErrorHandling};
use snafu::OptionExt;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use rust_parsing::error::{InvalidIoOperationsSnafu};
use snafu::ResultExt;
use std::env;
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
pub fn patch_data_argument() -> Result<(), ErrorHandling> {
    let path = env::current_dir()
        .context(InvalidIoOperationsSnafu)?;
    let args: Vec<String> = env::args().collect();
    let patch = get_patch_data(
        path.join(&args[1]),
        path,
    )?;
    for each in patch {
        let file = fs::read_to_string(&each.filename)
            .unwrap();
        println!("each: {:?}", &each.filename);
        let to_vec = FileExtractor::string_to_vector(&file);

        for obj in each.range{ 
            let item = &to_vec[obj.start-1..obj.end].join("\n");
            let parsed = &RustItemParser::parse_all_rust_items(item)
                .unwrap()[0];
            if parsed.object_type().unwrap() == "fn" { 
                println!("range at lines: {:?} object:\n{}", obj, item);
            }
        }
    }

    Ok(())
}

pub fn get_patch_data(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<Export>, ErrorHandling> {
    let export = patch_export_change(path_to_patch, relative_path)?;
    let mut export_difference: Vec<Export> = Vec::new();
    let mut vector_of_changed: Vec<Range<usize>> = Vec::new();
    for difference in export {
        let file = fs::read_to_string(&difference.filename).expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&file).expect("Failed to parse");
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

fn store_objects(
    relative_path: &PathBuf,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, Git2ErrorHandling> {
    let mut vec_of_surplus: Vec<FullDiffInfo> = Vec::new();
    let matched = match_patch_with_parse(&relative_path, patch_src)?;
    for change_line in &matched {
        if change_line.quantity == 1 {
            let list_of_unique_files =
                get_easy_hunk(patch_src, &change_line.change_at_hunk.filename())?;
            let path = relative_path.join(&change_line.change_at_hunk.filename());
            let file = fs::read_to_string(&path).expect("Failed read file");
            let parsed = RustItemParser::parse_all_rust_items(&file).expect("Failed to parse");
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
) -> Result<Vec<Difference>, ErrorHandling> {
    let mut change_in_line: Vec<usize> = Vec::new();
    let mut line_and_file: Vec<Difference> = Vec::new();
    let patch_text = fs::read(path_to_patch).expect("Failed to read patch file");
    let each_diff = store_objects(&relative_path, &patch_text).unwrap();
    for diff_hunk in &each_diff {
        let path_to_file = relative_path.to_owned().join(&diff_hunk.name);
        let file = fs::read_to_string(&path_to_file).expect("couldn't read file");
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
