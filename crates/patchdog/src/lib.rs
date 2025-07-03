use filesystem_parsing::parse_all_rust_items;
use filesystem_parsing::{ObjectRange, extract_function};
use snafu::Snafu;
use snafu::Whatever;
use snafu::prelude::*;
use std::path::Path;
#[derive(Debug, Snafu)]
pub enum HandlingError {
    #[snafu(display("Line start {} is greater than line end {}", line_start, line_end))]
    InvalidLineRange { line_start: usize, line_end: usize },

    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },

    #[snafu(display("Failed to parse file: {}", from))]
    ParseFile { from: Whatever },

    #[snafu(display("Line index {} is out of bounds", line_index))]
    LineOutOfBounds { line_index: usize },
}

pub fn seeker(line_index: usize, item: ObjectRange, from: &Path) -> Result<String, HandlingError> {
    let line_start = item.line_start().unwrap();
    let line_end = item.line_end().unwrap();
    ensure!(
        line_start <= line_index && line_end >= line_index,
        LineOutOfBoundsSnafu { line_index }
    );
    extract_function(from, &line_start, &line_end).with_whatever_context(|_| {
        format!(
            "Failed to extract object in line {} from file: {}",
            line_index,
            from.display()
        )
    })
}

pub fn receive_context(line_from: usize, file_path: &Path) -> Result<String, HandlingError> {
    let visited = parse_all_rust_items(file_path)
        .with_whatever_context(|_| format!("Failed to parse file: {}", file_path.display()))?;
    for item in visited {
        let found = seeker(line_from, item, file_path).with_whatever_context(|_| {
            format!(
                "Failed to extract object in line {} from file: {}",
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
