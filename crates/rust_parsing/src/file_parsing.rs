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

    fn string_to_vector(source: &str) -> Vec<String> {
        source.lines().map(|line| line.to_string()).collect()
    }

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
