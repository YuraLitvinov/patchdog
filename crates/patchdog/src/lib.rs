use filesystem_parsing::ErrorHandling;
use filesystem_parsing::LineOutOfBoundsSnafu;
use filesystem_parsing::parse_all_rust_items;
use filesystem_parsing::{ObjectRange, extract_function};
use snafu::prelude::*;
use std::path::Path;

pub fn seeker(line_index: usize, item: ObjectRange, from: &Path) -> Result<String, ErrorHandling> {
    let line_start = item.line_start().unwrap();
    let line_end = item.line_end().unwrap();
    ensure!(
        line_start <= line_index && line_end >= line_index,
        LineOutOfBoundsSnafu { line_index }
    );
    extract_function(from, &line_start, &line_end)
}

pub fn receive_context(line_from: usize, file_path: &Path) -> Result<String, ErrorHandling> {
    let visited = parse_all_rust_items(file_path);
    let visited = match visited {
        Ok(visited) => visited,
        Err(_) => {
            return Err(ErrorHandling::ErrorParsingFile {
                in_line: line_from,
                from: file_path.display().to_string(),
            });
        }
    };
    for item in visited {
        let found = seeker(line_from, item, file_path);
        if found.is_err() {
            continue;
        }
        return found;
    }
    Err(ErrorHandling::LineOutOfBounds {
        line_index: line_from,
    })
}
