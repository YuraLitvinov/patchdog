use crate::error::{
    ErrorHandling, InvalidIoOperationsSnafu, LineOutOfBoundsSnafu,
};
use crate::object_range::ObjectRange;
use snafu::{ResultExt, ensure};
use std::{fs::File, io::Write, path::PathBuf};
pub const REGEX: &str = r#"\{\s*"uuid"\s*:\s*"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",\s*"new_comment"\s*:\s*".*"\s*\}"#;
pub struct FileExtractor;
pub trait Files {
    fn check_for_valid_object(
        parsed: &[ObjectRange],
        line_number: usize,
    ) -> Result<bool, ErrorHandling>;
    fn export_object_preserving_comments(
        src: Vec<String>,
        from_line: usize,
        parsed: Vec<ObjectRange>,
    ) -> Result<String, ErrorHandling>;
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

/// Writes a modified vector of strings back to a file at a specific line index.
/// It inserts the `changed_element` string into the `source` vector at `line_index - 1` (to account for 0-based indexing).
/// Then, it overwrites the original file with the content of the modified `source` vector, writing each string on a new line.
///
/// # Arguments
///
/// * `path` - A `PathBuf` indicating the path to the file to be written.
/// * `source` - A mutable `Vec<String>` representing the lines of the file, which will be modified.
/// * `line_index` - The 1-based line number at which the `changed_element` should be inserted.
/// * `changed_element` - The `String` content to insert into the file.
///
/// # Returns
///
/// An `Ok(())` on successful file write.
/// An `Err(ErrorHandling::InvalidIoOperations)` if file creation or writing fails.
    fn write_to_vecstring(
        path: PathBuf,
        mut source: Vec<String>,
        line_index: usize,
        changed_element: String,
    ) -> Result<(), ErrorHandling> {
        source.insert(line_index.saturating_sub(1), changed_element);
        let mut file = File::create(path).context(InvalidIoOperationsSnafu)?;
        for each in &source {
            writeln!(file, "{each}").context(InvalidIoOperationsSnafu)?;
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

/// Extracts a code snippet from a source, preserving comments, based on a starting line and parsed object ranges.
/// It iterates through parsed `ObjectRange` items, using `seeker_for_comments` to find a valid range.
/// If a valid range is found, it extracts the lines from the source using `extract_by_line`.
/// The `new_previous` vector appears to track starting lines for search, ensuring comments between objects are considered.
///
/// # Arguments
///
/// * `src` - A `Vec<String>` representing the full source code, where each string is a line.
/// * `from_line` - The starting line number to begin the search for an object.
/// * `parsed` - A `Vec<ObjectRange>` representing the parsed Rust items with their line ranges.
///
/// # Returns
///
/// A `Result<String, ErrorHandling>`:
/// - `Ok(String)`: The extracted code snippet as a single string, including comments within the range.
/// - `Err(ErrorHandling::LineOutOfBounds)`: If no valid object range can be found containing or extending from `from_line`.
    fn export_object_preserving_comments(
        src: Vec<String>,
        from_line: usize,
        parsed: Vec<ObjectRange>,
    ) -> Result<String, ErrorHandling> {
        let mut new_previous: Vec<usize> = Vec::new();
        new_previous.push(1);
        let mut i = 0;
        for each in parsed {
            let found = seeker_for_comments(from_line, new_previous[i], each.line_ranges.end, &src);
            if found.is_err() {
                i += 1;
                let previous_end_line = each.line_ranges.end + 1;
                new_previous.push(previous_end_line);
                continue;
            }
            let extracted = extract_by_line(&src, &new_previous[i], &each.line_ranges.end);
            return Ok(extracted);
        }
        Err(ErrorHandling::LineOutOfBounds { line_number: 0 })
    }
}

fn seeker_for_comments(
    line_number: usize,
    line_start: usize,
    line_end: usize,
    src: &[String],
) -> Result<String, ErrorHandling> {
    ensure!(
        line_start <= line_number && line_end >= line_number,
        LineOutOfBoundsSnafu { line_number }
    );
    Ok(extract_by_line(src, &line_start, &line_end))
}
//Extracts a snippet from a file in regard to the snippet boundaries
fn extract_by_line(from: &[String], line_start: &usize, line_end: &usize) -> String {
    let line_start = line_start - 1;
    from[line_start..*line_end].join("\n")
}
