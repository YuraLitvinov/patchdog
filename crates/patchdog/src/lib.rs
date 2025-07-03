use filesystem_parsing::parse_all_rust_items;
use filesystem_parsing::{ObjectRange, extract_function};
use snafu::Snafu;
use snafu::prelude::*;
use std::path::Path;
#[derive(Debug, Snafu)]
pub enum HandlingError {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
    InvalidLineRange { line_start: usize, line_end: usize },
    ErrorParsingFile {in_line: usize, from: String},
    LineOutOfBounds { line_index: usize },
}

pub fn seeker(line_index: usize, item: ObjectRange, from: &Path) -> Result<String, HandlingError> {
    let line_start = item.line_start().unwrap();
    let line_end = item.line_end().unwrap();
    ensure!(
        line_start <= line_index && line_end >= line_index,
        LineOutOfBoundsSnafu { line_index }
    );
    extract_function(from, &line_start, &line_end)
        .with_whatever_context(|_| {
            format!(
                "{} {}",
                line_index,
                from.display()
            )
        })
}

pub fn receive_context(line_from: usize, file_path: &Path) -> Result<String, HandlingError> {
    let visited = parse_all_rust_items(file_path);
    let visited = match visited {
        Ok(visited) => visited,
        Err(_) => return Err(HandlingError::ErrorParsingFile { in_line: line_from, from: file_path.display().to_string()}),
    };
    for item in visited {
        let found = seeker(line_from, item, file_path)
            .with_whatever_context(|_| {
                format!(
                    "{} {}",
                    line_from,
                    file_path.display()
                )
            });
        if found.is_err() {
            continue;
        }
        return found;
    }
    Err(HandlingError::LineOutOfBounds {
        line_index: line_from,
    })
}
