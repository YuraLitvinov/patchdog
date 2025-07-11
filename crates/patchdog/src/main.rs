pub mod binding;
#[cfg(test)]
pub mod tests;
use crate::binding::rad_patch_export_change;
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use std::fs;
#[tokio::main]
async fn main() {
    let export = rad_patch_export_change(
        "/home/yurii-sama/Desktop/patchdog/patch.patch",
        "/home/yurii-sama/Desktop/patchdog/",
    );

    for difference in export {
        let file = fs::read_to_string(&difference.filename)
            .expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&file)
            .expect("Failed to parse");
        for each_parsed in &parsed {
            let range = each_parsed.line_start().unwrap()..each_parsed.line_end().unwrap();
            if difference.line.iter().any(|line| range.contains(line)) {
                println!("Found differentiating object at lines: {:?} in file {}", range, &difference.filename);
            }
        }
    }
}
