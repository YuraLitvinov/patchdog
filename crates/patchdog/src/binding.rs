use git_parsing::{Git2ErrorHandling, Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use std::fs;
pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: String,
    pub line: Vec<usize>,
}
#[derive(Debug)]
pub struct Export {
    pub filename: String,
    pub range: Vec<std::ops::Range<usize>>,

}
pub fn patch_interface(path_to_patch: &str, relative_path: &str) -> Vec<Export> {
    let export = patch_export_change(
        path_to_patch,
        relative_path,
    );
    let mut export_difference: Vec<Export> = Vec::new();
    let mut vector_of_change:  Vec<std::ops::Range<usize>> = Vec::new();
    for difference in export {
        let file = fs::read_to_string(&difference.filename)
            .expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&file)
            .expect("Failed to parse");
        for each_parsed in &parsed {
            let range = each_parsed.line_start().unwrap()..each_parsed.line_end().unwrap();
            if difference.line.iter().any(|line| range.contains(line)) {
                vector_of_change.push(range);
            }
        }
        export_difference.push(Export { range: vector_of_change.clone(), filename: difference.filename.clone() });
        vector_of_change.clear();
    }
    export_difference
}

fn store_objects(
    relative_path: &str,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, Git2ErrorHandling> {
    let mut vec_of_surplus: Vec<FullDiffInfo> = Vec::new();
    let matched = match_patch_with_parse(relative_path, patch_src).unwrap();
    for change_line in &matched {
        if change_line.quantity == 1 {
            let list_of_unique_files =
                get_easy_hunk(patch_src, &change_line.change_at_hunk.filename())?;
            let path = relative_path.to_string() + &change_line.change_at_hunk.filename();
            let file = fs::read_to_string(&path).expect("Failed read file");
            let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
            //.expect("Failed to parse");
            vec_of_surplus.push(FullDiffInfo {
                name: change_line.change_at_hunk.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            });
        }
    }

    Ok(vec_of_surplus)
}
//Absolute path is suggested, as there is some issue with relative
fn patch_export_change(path_to_patch: &str, relative_path: &str) -> Vec<Difference> {
    let mut change_in_line: Vec<usize> = Vec::new();
    let mut line_and_file: Vec<Difference> = Vec::new();
    let patch_text = fs::read(path_to_patch).expect("Failed to read patch file");
    let each_diff = store_objects(relative_path, &patch_text).unwrap();
    for diff_hunk in &each_diff {
        let path_to_file = relative_path.to_owned() + &diff_hunk.name;
        let file = fs::read_to_string(&path_to_file).expect("couldn't read file");
        let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
        let path = &path_to_file;

        for each in diff_hunk.hunk.clone() {
            let parsed_in_diff = parsed.clone();
            if FileExtractor::check_for_valid_object(&parsed_in_diff, each.get_line()).unwrap()
            {
                continue;
            }
            change_in_line.push(each.get_line());
        }
        line_and_file.push(Difference {
            filename: path.to_string(),
            line: change_in_line.clone(),
        });
        change_in_line.clear();
    }
    line_and_file
}