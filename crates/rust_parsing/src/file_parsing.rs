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

impl Files for FileExtractor {
/// Inserts a new line of code or comment into a vector representing file content at a specified line index, and then writes the entire modified content back to the original file.
/// This function is essential for programmatic modification of source files, ensuring that new content is placed precisely within the existing structure.
/// It handles the creation or overwriting of the file and ensures proper line-by-line writing, making it a robust utility for file manipulation.
///
/// # Arguments
///
/// * `path` - The `PathBuf` specifying the file to be written to.
/// * `source` - A mutable `Vec<String>` representing the lines of the file content, which will be modified in-place.
/// * `line_index` - The 1-based line number where `changed_element` should be inserted.
/// * `changed_element` - The `String` content to be inserted into the file.
///
/// # Returns
///
/// A `Result<(), ErrorHandling>`:
/// - `Ok(())`: If the write operation completes successfully.
/// - `Err(ErrorHandling)`: If an I/O error occurs during file creation or writing.
    /// Inserts a given `changed_element` string into a vector of source code lines at a specified `line_index` and then writes the modified content back to the original file path.
    /// This utility function is crucial for precisely inserting new lines of code or comments into an existing file structure.
    /// It handles file creation/overwriting and ensures proper line-by-line writing to the file system.
    ///
    /// # Arguments
    ///
    /// * `path` - The `PathBuf` to the file that will be written to.
    /// * `source` - A mutable `Vec<String>` representing the lines of the file content, which will be modified in-place.
    /// * `line_index` - The 1-based line number at which the `changed_element` should be inserted.
    /// * `changed_element` - The `String` content to insert into the file.
    ///
    /// # Returns
    ///
    /// A `Result<(), ErrorHandling>` indicating success or failure of the write operation.
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

/// Converts a multi-line string into a vector of individual strings, where each element in the vector corresponds to a single line from the original input.
/// This function is useful for processing text content line by line, such as parsing configuration files or source code.
/// It iterates through the input string's lines and collects them into a new `Vec<String>`.
///
/// # Arguments
///
/// * `source` - A string slice (`&str`) containing the multi-line content.
///
/// # Returns
///
/// A `Vec<String>`: A vector where each string represents a line from the input `source`.
    fn string_to_vector(source: &str) -> Vec<String> {
        source.lines().map(|line| line.to_string()).collect()
    }

/// Inserts a new string into a vector of source code lines, automatically preserving the indentation of the first line to maintain code formatting.
/// This function calculates the leading whitespace from the initial line of the source and applies it to the new content before insertion.
/// The `push_where` parameter determines whether the new content is prepended to the beginning or appended to the end of the `source_clone` vector.
///
/// # Arguments
///
/// * `str_source` - A slice of `String`s representing the original source code lines.
/// * `push` - The `String` content to be inserted.
/// * `push_where` - A `bool` flag: `true` to insert at the beginning, `false` to insert at the end.
///
/// # Returns
///
/// A `Result<Vec<String>, ErrorHandling>`:
/// - `Ok(Vec<String>)`: A new vector of strings with the `push` content inserted and correctly indented.
/// - `Err(ErrorHandling)`: If `str_source` is empty or whitespace calculation fails.
    /// Inserts a given string into a vector of strings while preserving the original indentation.
    /// It calculates the whitespace from the first line of `str_source` and prepends it to `push`.
    /// The `push_where` boolean determines whether to insert the new string at the beginning (`true`) or the end (`false`) of the `source_clone` vector.
    ///
    /// # Arguments
    ///
    /// * `str_source` - A slice of `String`s representing the original source lines.
    /// * `push` - The `String` content to be inserted.
    /// * `push_where` - A `bool` indicating where to insert: `true` for the beginning, `false` for the end.
    ///
    /// # Returns
    ///
    /// A `Result<Vec<String>, ErrorHandling>`:
    /// - `Ok(Vec<String>)`: A new vector of strings with the `push` content inserted and correctly indented.
    /// - `Err(ErrorHandling::LineOutOfBounds)`: If `str_source` is empty and a whitespace cannot be determined.
    /// - `Err(ErrorHandling)`: If `remove_whitespace` fails.
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

/// Checks if a given `line_number` falls within the line range of any `ObjectRange` present in a slice of parsed Rust items.
/// This function is primarily used to determine if a specific line of code is part of an already identified Rust entity, such as a function, struct, or module.
/// It iterates through the provided `ObjectRange`s and compares the `line_number` against their `start` and `end` line ranges.
///
/// # Arguments
///
/// * `parsed` - A slice of `ObjectRange` structs, each defining a Rust item's line boundaries.
/// * `line_number` - The 1-based line number to check for containment within an object's range.
///
/// # Returns
///
/// A `Result<bool, ErrorHandling>`:
/// - `Ok(false)`: If the `line_number` is found to be within the range of at least one `ObjectRange`.
/// - `Ok(true)`: If the `line_number` is not contained within any of the provided `ObjectRange`s.
/// - `Err(ErrorHandling)`: This function currently does not explicitly return errors, but the signature allows for future error propagation.
    /// Checks if a given `line_number` falls within the line range of any `ObjectRange` in the `parsed` slice.
    /// This function is typically used to determine if a specific line belongs to an existing parsed Rust item (like a function, struct, etc.).
    ///
    /// # Arguments
    ///
    /// * `parsed` - A slice of `ObjectRange` structs, each representing a parsed Rust item with its line range.
    /// * `line_number` - The 1-based line number to check.
    ///
    /// # Returns
    ///
    /// A `Result<bool, ErrorHandling>`:
    /// - `Ok(false)`: If the `line_number` is found within the range of any `ObjectRange`.
    /// - `Ok(true)`: If the `line_number` is not found within the range of any `ObjectRange`.
    /// - `Err(ErrorHandling)`: Currently, no errors are explicitly returned, but the signature indicates potential for error propagation.
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
