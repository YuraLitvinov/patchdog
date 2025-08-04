use gemini_rust::Error;
use git_parsing::Git2ErrorHandling;
use snafu::Snafu;
use yaml_rust2::ScanError;
use std::{env::VarError, num::ParseIntError, path::PathBuf};
use syn;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ErrorHandling {
    #[snafu(display("Unable to read {line_index} from file {file_name}"))]
    BadFile {
        line_index: usize,
        file_name: String,
    },
    #[snafu(display("invalid lines {line_start} {line_end}"))]
    InvalidLineRange {
        line_start: usize,
        line_end: usize,
    },
    #[snafu(display("{in_line} from {from}"))]
    ErrorParsingFile {
        in_line: usize,
        from: String,
    },
    #[snafu(display("{line_number}"))] 
    LineOutOfBounds {
        line_number: usize,
    },
    #[snafu(display("{source}"))]
    InvalidIoOperations {
        source: std::io::Error,
    },
    #[snafu(display("{source}"))]
    StdVarError {
        source: VarError,
    },
    #[snafu(display("{source}"))]
    GeminiRustError {
        source: gemini_rust::Error,
    },
    #[snafu(display("{source} in {file_path:#?}"))]   
    InvalidReadFileOperation {
        source: std::io::Error,
        file_path: PathBuf,
    },
    #[snafu(display("{source:#?} in {str_source:#?}"))]
    InvalidItemParsing {
        source: syn::Error,
        str_source: PathBuf,
    },
        #[snafu(display("{source:#?}"))]
    InvalidRustParse {
        source: syn::Error,
    },
#[snafu(display("Couldn't seek object at line: {line_number}"))]
    SeekerFailed {
        line_number: usize,
    },
    ExportObjectFailed {
        line_number: usize,
        src: String,
    },
    NotFunction,
    #[snafu(display("couldn't get line"))]
    CouldNotGetLine,
    CouldNotGetObject {
        err_kind: String,
    },
    #[snafu(display("{source}"))] 
    SerdeError {
        source: serde_json::error::Error,
    },
    #[snafu(display("{source}"))] 
    UuidError {
        source: uuid::Error,
    },
    #[snafu(display("{source}"))]    
    YamlError {
        source: yaml_rust2::scanner::ScanError,
    },
    #[snafu(display("{source}"))] 
    ParseErr {
        source: ParseIntError,
    }
}

#[derive(Debug)]
pub enum ErrorBinding {
    GitParsing(Git2ErrorHandling),
    RustParsing(ErrorHandling),
}

impl From<Git2ErrorHandling> for ErrorBinding {
/// Creates a new `ErrorBinding` from a given `Git2ErrorHandling`.
///
/// # Arguments
///
/// * `git`: The `Git2ErrorHandling` to convert.
///
/// # Returns
///
/// A new `ErrorBinding`.
    fn from(git: Git2ErrorHandling) -> Self {
        ErrorBinding::GitParsing(git)
    }
}

impl From<ErrorHandling> for ErrorBinding {
/// Creates a new `ErrorBinding` from a given `ErrorHandling`.
///
/// # Arguments
///
/// * `rust`: The `ErrorHandling` to convert.
///
/// # Returns
///
/// A new `ErrorBinding`.
    fn from(rust: ErrorHandling) -> Self {
        ErrorBinding::RustParsing(rust)
    }
}

impl From<CouldNotGetLineSnafu> for ErrorHandling {
    fn from(e: CouldNotGetLineSnafu) -> Self {
        return e.into()
    }
}

impl From<CouldNotGetLineSnafu> for ErrorBinding {
    fn from(e: CouldNotGetLineSnafu) -> Self {
        return e.into()
        
    }
}

impl From<std::io::Error> for ErrorHandling {
    fn from(e: std::io::Error) -> Self {
        return ErrorHandling::InvalidIoOperations { source: e }
    }
}
impl From<std::io::Error> for ErrorBinding {
    fn from(e: std::io::Error) -> Self {
        return ErrorBinding::RustParsing(ErrorHandling::InvalidIoOperations { source: e })
    }
}

impl From<ScanError> for ErrorHandling {
    fn from(e: ScanError) -> Self {
        return ErrorHandling::YamlError { source: e }
    }
}
impl From<VarError> for ErrorHandling {
    fn from(e: VarError) -> Self {
        return ErrorHandling::StdVarError { source: e }
    }
}

impl From<ParseIntError> for ErrorHandling {
    fn from(e: ParseIntError) -> Self {
     return ErrorHandling::ParseErr { source: e }
    }
    
}
impl From<Error> for ErrorHandling {
    fn from(e: Error) -> Self {
        return ErrorHandling::GeminiRustError { source: e }
    }
}
impl From<serde_json::Error> for ErrorHandling {
    fn from(e: serde_json::Error) -> Self {
        return ErrorHandling::SerdeError { source: e }
    }
    
}
impl From<syn::Error> for ErrorHandling {
    fn from(e: syn::Error) -> Self {
        return  ErrorHandling::InvalidRustParse { source: e } 
    }
}