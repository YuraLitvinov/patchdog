use std::fs;
pub mod binding;
pub mod tests;

use binding::store_objects;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
#[tokio::main]
async fn main() {
    read_patch_export_change(
        "/home/yurii-sama/Desktop/patchdog/patch.patch",
        "/home/yurii-sama/Desktop/patchdog/",
    );
}
//Absolute path is suggested, as there is some issue with relative
fn read_patch_export_change(path_to_patch: &str, relative_path: &str) {
    let patch_text = fs::read(path_to_patch).expect("Failed to read patch file");
    let each_diff = store_objects(relative_path, &patch_text).unwrap();
    for diff_hunk in &each_diff {
        let path = relative_path.to_owned() + &diff_hunk.name;
        println!("path: {}", path);
        let file = fs::read_to_string(path).expect("couldn't read file");
        let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
        for each in diff_hunk.hunk.clone() {
            if each.get_change() == "Remove" {
                continue;
            }
            println!("checked at line: {:?}", each.get_line());
            if FileExtractor::check_for_not_comment(&parsed, each.get_line()).unwrap() {
                println!("change outside of code block");
                continue;
            }

            let extracted = FileExtractor::return_match(each.get_line(), parsed.clone()).unwrap();

            println!("{:?}", extracted);
        }
    }
}
