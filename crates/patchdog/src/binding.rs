use git_parsing::{Git2ErrorHandling, Hunk, get_easy_hunk, match_patch_with_parse};
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use std::fs;

#[derive(Debug)]
pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub fn store_objects(
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
