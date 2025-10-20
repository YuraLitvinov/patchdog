use crate::error::{ErrorHandling, InvalidIoOperationsSnafu};
use crate::object_range::ObjectRange;
use crate::rust_parser::RustItemParser;
use crate::rust_parser::RustParser;
use snafu::ResultExt;
use std::{fs::File, io::Write, path::PathBuf};
use textdistance::nstr::damerau_levenshtein;
use tracing::Level;
// This REGEX is used outside of file_parsing and inside fallback_repair for JSON parsing
pub const REGEX: &str = r#"\{\s*"uuid"\s*:\s*"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",\s*"new_comment"\s*:\s*".*"\s*\}"#;
pub struct FsOperations;
pub trait Files {
    fn check_for_valid_object(parsed: &[&ObjectRange], line_number: usize) -> bool;

    fn string_to_vector(source: &str) -> Vec<String>;

    fn write_with_remove(
        path: PathBuf,
        source: Vec<String>,
        line_number: usize,
        changed_element: String,
    ) -> Result<(), ErrorHandling>;
}

impl Files for FsOperations {
    /// Converts a multi-line string slice into a `Vec<String>`, where each element of the vector represents a single line from the input string. This function is a straightforward utility for breaking down file content into manageable line-by-line components.
    ///
    /// # Arguments
    /// * `source` - A string slice (`&str`) containing the multi-line text.
    ///
    /// # Returns
    /// A `Vec<String>` where each `String` is a line from the input `source`.
    fn string_to_vector(source: &str) -> Vec<String> {
        source.lines().map(|line| line.to_string()).collect()
    }

    /// Checks if a given `line_number` falls within any of the provided `ObjectRange` items. This utility function helps determine if a specific line of code is part of a recognized structural element (like a function or struct) within the parsed code.
    ///
    /// # Arguments
    /// * `parsed` - A slice of `ObjectRange` representing the parsed code objects and their line ranges.
    /// * `line_number` - The `usize` line number to check.
    ///
    /// # Returns
    /// A `bool` which is `false` if the line number is within an object's range, `true` otherwise.
    fn check_for_valid_object(parsed: &[&ObjectRange], line_number: usize) -> bool {
        for each in parsed {
            if each.line_ranges.start <= line_number && line_number <= each.line_ranges.end {
                return false;
            }
        }
        true
    }

    /// Modifies a file by replacing or inserting new `changes` (comments) for a specific Rust function at a given `line_number`. It identifies the target function, determines if existing documentation needs replacement, or if new comments should be inserted, and then applies the `changes` while preserving original indentation.
    ///
    /// # Arguments
    /// * `path` - The `PathBuf` to the file being modified.
    /// * `source` - A `Vec<String>` representing the lines of the file content.
    /// * `line_number` - The 1-indexed line number where the change is located, typically within the function to be documented.
    /// * `changes` - The new comment or code string to be inserted.
    ///
    /// # Returns
    /// A `Result<(), ErrorHandling>` indicating success or failure of the write operation.
    fn write_with_remove(
        path: PathBuf,
        mut source: Vec<String>,
        line_number: usize,
        changes: String,
    ) -> Result<(), ErrorHandling> {
        let parsed = RustItemParser::parse_all_rust_items(&source.join("\n"))?;
        let functions = parsed
            .into_iter()
            .filter_map(|each| {
                if each.names.type_name == "fn" {
                    Some(each)
                } else {
                    None
                }
            })
            .collect::<Vec<ObjectRange>>();
        let function_at_line_number = functions
            .iter()
            .find(|each| !FsOperations::check_for_valid_object(&[*each], line_number));

        if let Some(func) = function_at_line_number {
            let function_range = source[func.line_start() - 1..func.line_end()].to_vec();
            let parse_items_within_function_range =
                RustItemParser::parse_all_rust_items(&function_range.join("\n"))?;

            //Gatekeep against functions with no prior documentation
            let comments = parse_items_within_function_range
                .into_iter()
                .filter_map(|each| {
                    if each.names.name == "///" {
                        Some(each)
                    } else {
                        None
                    }
                })
                .collect::<Vec<ObjectRange>>();
            if comments.is_empty() {
                tracing::event!(Level::INFO, "{}", comments.len());
                let rfc_changes = push_to_vector(&function_range, changes)?;

                write_to_vecstring(path, source, func.line_start(), rfc_changes)?;

                return Ok(());
            }
            let mut function_no_comment = function_range;

            if let Some(first_value) = comments.first()
                && let Some(last_value) = comments.last()
            {
                let comment = &source[func.line_start() - 1..func.line_end()].join("\n");

                function_no_comment.drain(first_value.line_start() - 1..last_value.line_end());

                tracing::event!(Level::INFO, "{function_at_line_number:?}");

                source.insert(func.line_end(), function_no_comment.join("\n"));
                source.drain(func.line_start() - 1..func.line_end());

                // Included whitespaces are necessary to suffice RFC 1574 convention
                let rfc_changes = push_to_vector(&function_no_comment, changes)?;

                if damerau_levenshtein(comment, &rfc_changes) < 6.0 / 7.0 {
                    write_to_vecstring(path, source, func.line_start(), rfc_changes)?;
                }

                return Ok(());
            }
        }

        Ok(())
    }
}

/// Writes a modified vector of strings, representing file content, to a specified file on disk. It inserts the `changes` string at the given `line_number` (1-indexed) into the `source` vector.
/// The function then creates or overwrites the file at `path` and writes each line from the updated vector, followed by a newline.
///
/// # Arguments
/// * `path` - The `PathBuf` to the target file where content will be written.
/// * `source` - A mutable `Vec<String>` representing the lines of the file content.
/// * `line_number` - The 1-indexed line number at which to insert the new changes.
/// * `changes` - The string containing the content to be inserted.
///
/// # Returns
/// A `Result<(), ErrorHandling>` indicating success or failure of the file write operation.
fn write_to_vecstring(
    path: PathBuf,
    mut source: Vec<String>,
    line_number: usize,
    changes: String,
) -> Result<(), ErrorHandling> {
    let path = path.as_path();
    source.insert(line_number.saturating_sub(1), changes);
    let mut file = File::create(path).context(InvalidIoOperationsSnafu { path: &path })?;
    for each in &source {
        writeln!(file, "{each}").context(InvalidIoOperationsSnafu { path: &path })?;
    }
    Ok(())
}

/// Formats a multi-line string (`push`) by prepending the leading whitespace from the first line of `str_source` to each of its lines. This ensures that the new content maintains the original indentation when inserted into a code block.
///
/// # Arguments
/// * `str_source` - A slice of strings, typically representing the lines of an existing code block, used to determine indentation.
/// * `push` - The multi-line string content to be formatted.
///
/// # Returns
/// A `Result<String, ErrorHandling>` containing the formatted multi-line string with preserved indentation, or an `ErrorHandling` if `str_source` is empty.
fn push_to_vector(str_source: &[String], push: String) -> Result<String, ErrorHandling> {
    let whitespace = str_source
        .first()
        .ok_or(ErrorHandling::LineOutOfBounds { line_number: 0 })?
        .chars()
        .take_while(|w| w.is_whitespace())
        .collect::<String>();

    let affect_all_lines = FsOperations::string_to_vector(&push)
        .into_iter()
        .map(|each| whitespace.clone() + &each)
        .collect::<Vec<String>>()
        .join("\n");

    Ok(affect_all_lines)
}
