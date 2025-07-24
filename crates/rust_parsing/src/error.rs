use std::{env::VarError, path::PathBuf};

use snafu::Snafu;
use syn;
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ErrorHandling {
    #[snafu(display("Unable to read {line_index} from file {file_name}"))]
    BadFile {
        line_index: usize,
        file_name: String,
    },
    InvalidLineRange {
        line_start: usize,
        line_end: usize,
    },
    ErrorParsingFile {
        in_line: usize,
        from: String,
    },

    LineOutOfBounds {
        line_number: usize,
    },
    InvalidIoOperations {
        source: std::io::Error,
    },
    StdVarError {
        source: VarError
    },
    GeminiRustError {
        source: gemini_rust::Error
    },
    InvalidReadFileOperation {
        source: std::io::Error,
        file_path: PathBuf,
    },
    InvalidItemParsing {
        source: syn::Error,
        str_source: PathBuf,
    },
    SeekerFailed {
        line_number: usize,
    },
    ExportObjectFailed {
        line_number: usize,
        src: String,
    },
    NotFunction,
    CouldNotGetLine,
    CouldNotGetObject {err_kind: String },
}
