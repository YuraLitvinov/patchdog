use git_parsing::{get_filenames, git_get_hunks, read_non_repeting_functions};
use git2::Diff;
use rust_parsing::{export_object, parse_all_rust_items, string_to_vector};
use std::fs;
pub mod tests;
#[tokio::main]
async fn main() {
    match_patch_with_parse("../../patch.patch");
}
fn match_patch_with_parse(src: &str) {
    let patch_text = fs::read(src).expect("Failed to read patch file");
    let read = read_non_repeting_functions(&patch_text).expect("Failed to read");
    let diff = Diff::from_buffer(&patch_text).unwrap();
    let changed = get_filenames(&diff).expect("Unwrap on get_filenames failed");
    let mut hunks = git_get_hunks(diff, changed).expect("Error?");

    hunks.sort_by(|a, b| a.2.cmp(&b.2));
    for read in read {
        for each in hunks.clone().into_iter() {
            let path = "../../".to_string() + &each.2;
            let file = fs::read_to_string(&path).expect("failed to read");
            let file_vector = string_to_vector(&file);
            let parsed = parse_all_rust_items(file).expect("failed to parse");
            let what_change_occured = match each.0 {
                "Add" => {
                    println!("Added: \n {}", &read);
                    export_object(each.1, parsed, &file_vector).unwrap()
                }

                "Remove" => {
                    println!("Removed line number: {} \n{} ", each.1, &read);
                    "".to_string()
                }
                "Modify" => {
                    println!("Modified: \n {}", &each.2);
                    export_object(each.1, parsed, &file_vector).expect("Error on Modify")
                }
                _ => "".to_string(),
            };
            println!("{}", what_change_occured);
        }
    }
}
