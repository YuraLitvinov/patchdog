use crate::error::{ErrorHandling, InvalidIoOperationsSnafu};
use crate::object_range::ObjectRange;
use snafu::ResultExt;
use std::{fs::File, io::Write, path::PathBuf};
pub const REGEX: &str = r#"\{\s*"uuid"\s*:\s*"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",\s*"new_comment"\s*:\s*".*"\s*\}"#;
pub struct FileExtractor;
pub trait Files {
    fn check_for_valid_object(
        parsed: &[ObjectRange],
        line_number: usize,
    ) -> Result<bool, ErrorHandling>;

    fn string_to_vector(source: &str) -> Vec<String>;

    fn push_to_vector(
        str_source: &[String],
        push: String,
        push_path: bool,
    ) -> Result<Vec<String>, ErrorHandling>;
    //Representing file as a vector of lines is generally the most effective practise
    fn write_to_vecstring(
        path: PathBuf,
        source: Vec<String>,
        line_index: usize,
        changed_element: String,
    ) -> Result<(), ErrorHandling>;
}

/// Provides implementations for the `Files` trait, offering utilities for manipulating file content as vectors of strings. This `impl` block includes methods for writing a vector of strings to a file at a specific line index, converting a string slice into a vector of strings (line by line), and pushing a new string to the beginning or end of a vector while preserving indentation.
/// It also includes a utility to check if a given line number falls within a parsed code object's range. These methods collectively enable robust file content management and analysis.
///
/// This implementation block provides core utilities for file manipulation and parsing, crucial for features like applying patch changes or analyzing code structure.
/// It handles potential I/O errors and preserves formatting where applicable.
impl Files for FileExtractor {

/// Inserts a `changed_element` string into a `Vec<String>` representation of a file at a specific `line_index` and then writes the modified content back to the original file path. This function is designed to apply changes to a file by modifying its in-memory line representation and then persisting these changes.
/// It handles file creation and writing, ensuring that the updated content is correctly saved. The `line_index` is adjusted to be 1-based for user convenience, but internally adjusted for 0-based vector indexing.
///
/// # Arguments
///
/// * `path` - A `PathBuf` representing the path to the file to be modified.
/// * `source` - A mutable `Vec<String>` containing the lines of the file.
/// * `line_index` - The 1-based `usize` line number where the `changed_element` should be inserted.
/// * `changed_element` - The `String` content to be inserted into the file.
///
/// # Returns
///
/// A `Result<(), ErrorHandling>` indicating success or an `ErrorHandling` if file operations fail.
    fn write_to_vecstring(
        path: PathBuf,
        mut source: Vec<String>,
        line_index: usize,
        changed_element: String,
    ) -> Result<(), ErrorHandling> {
        source.insert(line_index.saturating_sub(1), changed_element);
        let mut file =
            File::create(path.clone()).context(InvalidIoOperationsSnafu { path: path.clone() })?;
        for each in &source {
            writeln!(file, "{each}").context(InvalidIoOperationsSnafu { path: path.clone() })?;
        }
        Ok(())
    }

/// Converts a multi-line string slice into a `Vec<String>`, where each element of the vector represents a single line from the input string. This function is a straightforward utility for breaking down file content into manageable line-by-line components.
/// It simplifies processing textual data by providing a convenient way to iterate, modify, or analyze individual lines of a file.
///
/// # Arguments
///
/// * `source` - A string slice (`&str`) containing the multi-line text.
///
/// # Returns
///
/// A `Vec<String>` where each `String` is a line from the input `source`.
    fn string_to_vector(source: &str) -> Vec<String> {
        source.lines().map(|line| line.to_string()).collect()
    }

/// Inserts a new string (`push`) into a vector of strings (`str_source`), either at the beginning or the end, while preserving the indentation of the first line. This function is useful for adding new lines of code or comments to a file's content represented as a vector of strings, maintaining consistent formatting.
/// It intelligently extracts the leading whitespace from the first line of `str_source` and prepends it to the `push` string before insertion.
///
/// # Arguments
///
/// * `str_source` - A slice of `String` representing the current lines of code.
/// * `push` - The `String` content to be inserted.
/// * `push_where` - A `bool` flag: `true` to insert at the beginning (index 0), `false` to insert at the end.
///
/// # Returns
///
/// A `Result<Vec<String>, ErrorHandling>` containing the modified `Vec<String>` with the new content inserted, or an `ErrorHandling` if the `str_source` is empty.
    fn push_to_vector(
        str_source: &[String],
        push: String,
        push_where: bool,
    ) -> Result<Vec<String>, ErrorHandling> {
        let mut source_clone = str_source.to_owned(); //We do this, so the str_source stays immutable
        let whitespace = &source_clone
            .first()
            .ok_or(ErrorHandling::LineOutOfBounds { line_number: 0 })?
            .chars()
            .take_while(|w| w.is_whitespace())
            .collect::<String>();
        //whitespace variable preserves formatting, push is the value that has to be inserted
        let push_preserving = whitespace.to_owned() + &push;
        if push_where {
            source_clone.insert(0_usize, push_preserving);
        } else {
            source_clone.insert(source_clone.len(), push_preserving);
        }
        Ok(source_clone)
    }


/// Checks if a given `line_number` falls within any of the provided `ObjectRange` items. This utility function helps determine if a specific line of code is part of a recognized structural element (like a function or struct) within the parsed code.
/// It iterates through a slice of `ObjectRange` objects and returns `false` as soon as it finds a range that contains the `line_number`, indicating that the line is validly part of an object. If no such object is found, it returns `true`.
///
/// # Arguments
///
/// * `parsed` - A slice of `ObjectRange` representing the parsed code objects and their line ranges.
/// * `line_number` - The `usize` line number to check.
///
/// # Returns
///
/// A `Result<bool, ErrorHandling>` which is `false` if the line number is within an object's range, `true` otherwise, or an `ErrorHandling` if an internal error occurs.
    fn check_for_valid_object(
        parsed: &[ObjectRange],
        line_number: usize,
    ) -> Result<bool, ErrorHandling> {
        for each in parsed {
            if each.line_ranges.start <= line_number && line_number <= each.line_ranges.end {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
