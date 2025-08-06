use gemini_rust::Error;
use git_parsing::Git2ErrorHandling;
use snafu::Snafu;
use std::{env::VarError, num::ParseIntError, path::PathBuf};
use syn;
use yaml_rust2::ScanError;

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
    },
}

#[derive(Debug)]
pub enum ErrorBinding {
    GitParsing(Git2ErrorHandling),
    RustParsing(ErrorHandling),
}

impl From<Git2ErrorHandling> for ErrorBinding {

/// Converts a `Git2ErrorHandling` error into an `ErrorBinding::GitParsing` variant.
///
/// This function facilitates the conversion of errors originating from the `git2` library's
/// error handling into a more generalized `ErrorBinding` type, specifically categorizing
/// them as parsing-related issues.
///
/// # Arguments
///
/// * `git` - The `Git2ErrorHandling` error to be converted.
///
/// # Returns
///
/// Returns an `ErrorBinding::GitParsing` enum variant, encapsulating the provided
/// `Git2ErrorHandling` error.
    fn from(git: Git2ErrorHandling) -> Self {
        ErrorBinding::GitParsing(git)
    }
}

impl From<ErrorHandling> for ErrorBinding {

/// Implements the `From` trait to convert an `ErrorHandling` into an `ErrorBinding::RustParsing` error.
/// This provides a standardized way to integrate general Rust parsing errors into the broader application error handling system.
///
/// # Arguments
///
/// * `rust` - The `ErrorHandling` error to convert.
///
/// # Returns
///
/// An `ErrorBinding::RustParsing` variant containing the original `ErrorHandling`.
    fn from(rust: ErrorHandling) -> Self {
        ErrorBinding::RustParsing(rust)
    }
}

impl From<CouldNotGetLineSnafu> for ErrorHandling {
/// Implements the `From` trait to convert a `CouldNotGetLineSnafu` error into an `ErrorHandling`.
/// It achieves this by first converting `CouldNotGetLineSnafu` into an `ErrorHandling` through an intermediate step.
/// This allows `CouldNotGetLineSnafu` errors to be seamlessly integrated into the `ErrorHandling` error type.
///
/// # Arguments
///
/// * `e` - The `CouldNotGetLineSnafu` error to convert.
///
/// # Returns
///
/// An `ErrorHandling` error.
    fn from(e: CouldNotGetLineSnafu) -> Self {
        let e: ErrorHandling = e.into();
        e
    }
}

impl From<CouldNotGetLineSnafu> for ErrorBinding {
/// Implements the `From` trait to convert a `CouldNotGetLineSnafu` error into an `ErrorBinding`.
/// It achieves this by first converting `CouldNotGetLineSnafu` into an `ErrorBinding` through an intermediate step.
/// This allows `CouldNotGetLineSnafu` errors to be seamlessly integrated into the `ErrorBinding` error type.
///
/// # Arguments
///
/// * `e` - The `CouldNotGetLineSnafu` error to convert.
///
/// # Returns
///
/// An `ErrorBinding` error.
    fn from(e: CouldNotGetLineSnafu) -> Self {
        let e: ErrorBinding = e.into();
        e
    }
}

impl From<std::io::Error> for ErrorHandling {
/// Implements the `From` trait to convert a `std::io::Error` into an `ErrorHandling::InvalidIoOperations` error.
/// This provides a standardized way to encapsulate I/O related errors within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `std::io::Error` to convert.
///
/// # Returns
///
/// An `ErrorHandling::InvalidIoOperations` variant containing the original `std::io::Error` as its source.
    fn from(e: std::io::Error) -> Self {
        ErrorHandling::InvalidIoOperations { source: e }
    }
}
impl From<std::io::Error> for ErrorBinding {
/// Implements the `From` trait to convert a `std::io::Error` into an `ErrorBinding::RustParsing` error.
/// This conversion is done by first wrapping the `std::io::Error` in an `ErrorHandling::InvalidIoOperations`,
/// and then wrapping that into the `ErrorBinding::RustParsing` variant.
/// This allows I/O errors encountered during Rust parsing to be consistently handled within the `ErrorBinding` system.
///
/// # Arguments
///
/// * `e` - The `std::io::Error` to convert.
///
/// # Returns
///
/// An `ErrorBinding::RustParsing` variant, containing an `ErrorHandling::InvalidIoOperations` error.
    fn from(e: std::io::Error) -> Self {
        ErrorBinding::RustParsing(ErrorHandling::InvalidIoOperations { source: e })
    }
}

impl From<ScanError> for ErrorHandling {
/// Implements the `From` trait to convert a `yaml_rust::ScanError` into an `ErrorHandling::YamlError`.
/// This provides a standardized way to encapsulate YAML scanning errors within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `yaml_rust::ScanError` to convert.
///
/// # Returns
///
/// An `ErrorHandling::YamlError` variant containing the original `ScanError` as its source.
    fn from(e: ScanError) -> Self {
        ErrorHandling::YamlError { source: e }
    }
}
impl From<VarError> for ErrorHandling {
/// Implements the `From` trait to convert a `std::env::VarError` into an `ErrorHandling::StdVarError`.
/// This provides a standardized way to encapsulate environment variable errors within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `std::env::VarError` to convert.
///
/// # Returns
///
/// An `ErrorHandling::StdVarError` variant containing the original `VarError` as its source.
    fn from(e: VarError) -> Self {
        ErrorHandling::StdVarError { source: e }
    }
}

impl From<ParseIntError> for ErrorHandling {
/// Implements the `From` trait to convert a `std::num::ParseIntError` into an `ErrorHandling::ParseErr`.
/// This provides a standardized way to encapsulate integer parsing errors within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `std::num::ParseIntError` to convert.
///
/// # Returns
///
/// An `ErrorHandling::ParseErr` variant containing the original `ParseIntError` as its source.
    fn from(e: ParseIntError) -> Self {
        ErrorHandling::ParseErr { source: e }
    }
}
impl From<Error> for ErrorHandling {
/// Implements the `From` trait to convert a `gemini_rust::Error` into an `ErrorHandling::GeminiRustError`.
/// This provides a standardized way to encapsulate errors originating from the `gemini-rust` crate within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `gemini_rust::Error` to convert.
///
/// # Returns
///
/// An `ErrorHandling::GeminiRustError` variant containing the original `gemini_rust::Error` as its source.
    fn from(e: Error) -> Self {
        ErrorHandling::GeminiRustError { source: e }
    }
}
impl From<serde_json::Error> for ErrorHandling {
/// Implements the `From` trait to convert a `serde_json::Error` into an `ErrorHandling::SerdeError`.
/// This provides a standardized way to encapsulate JSON serialization/deserialization errors within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `serde_json::Error` to convert.
///
/// # Returns
///
/// An `ErrorHandling::SerdeError` variant containing the original `serde_json::Error` as its source.
    fn from(e: serde_json::Error) -> Self {
        ErrorHandling::SerdeError { source: e }
    }
}
impl From<syn::Error> for ErrorHandling {
/// Implements the `From` trait to convert a `syn::Error` into an `ErrorHandling::InvalidRustParse`.
/// This provides a standardized way to encapsulate syntax analysis errors from the `syn` crate within the custom `ErrorHandling` enum.
///
/// # Arguments
///
/// * `e` - The `syn::Error` to convert.
///
/// # Returns
///
/// An `ErrorHandling::InvalidRustParse` variant containing the original `syn::Error` as its source.
    fn from(e: syn::Error) -> Self {
        ErrorHandling::InvalidRustParse { source: e }
    }
}
