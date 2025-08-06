use crate::error::{
    ErrorHandling, InvalidIoOperationsSnafu, LineOutOfBoundsSnafu, SeekerFailedSnafu,
};
use crate::object_range::ObjectRange;
use snafu::{ResultExt, ensure};
use std::{fs::File, io::Write, path::PathBuf};
//Advanced matching. This inefficient method is chosen due to error: look-around, including look-ahead and look-behind, is not supported
/*
pub const REGEX: &str = r#"\{\s*("uuid"\s*:\s*"[^"]*"\s*,\s*"fn_name"\s*:\s*"[^"]*"\s*,\s*"new_comment"\s*:\s*"[^"]*"
|\s*"uuid"\s*:\s*"[^"]*"\s*,\s*"new_comment"\s*:\s*"[^"]*"\s*,\s*"fn_name"\s*:\s*"[^"]*"
|\s*"fn_name"\s*:\s*"[^"]*"\s*,\s*"uuid"\s*:\s*"[^"]*"\s*,\s*"new_comment"\s*:\s*"[^"]*"
|\s*"fn_name"\s*:\s*"[^"]*"\s*,\s*"new_comment"\s*:\s*"[^"]*"\s*,\s*"uuid"\s*:\s*"[^"]*"
|\s*"new_comment"\s*:\s*"[^"]*"\s*,\s*"uuid"\s*:\s*"[^"]*"\s*,\s*"fn_name"\s*:\s*"[^"]*"
|\s*"new_comment"\s*:\s*"[^"]*"\s*,\s*"fn_name"\s*:\s*"[^"]*"\s*,\s*"uuid"\s*:\s*"[^"]*")\s*\}"#;
*/
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

    fn export_object(
        from_line_number: usize,
        visited: &[ObjectRange],
        src: &[String],
    ) -> Result<String, ErrorHandling>;

    fn string_to_vector(str_source: &str) -> Vec<String>;
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
    /// Writes a given vector of strings (representing file content) to a specified file path.
    /// Before writing, it inserts a `changed_element` string at a specific `line_index` (adjusting for 0-based indexing).
    /// Each string in the modified vector is written as a new line in the file.
    ///
    /// # Arguments
    ///
    /// * `path`: A `PathBuf` specifying the target file path.
    /// * `source`: A `Vec<String>` containing the lines of the file content.
    /// * `line_index`: The 1-based line number at which `changed_element` should be inserted.
    /// * `changed_element`: The `String` to be inserted into the file content.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok(())`) or an `ErrorHandling` if file creation or writing fails.
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

    //Assuming str_source can be of different size at runtime, there is sense to only include pushing at begin, represented by true and end, represented by false
    /// Inserts a new string (`push`) into a vector of strings (`str_source`) either at the beginning or at the end.
    /// It intelligently preserves the leading whitespace of the first line of the original source to maintain consistent indentation for the inserted string.
    ///
    /// # Arguments
    ///
    /// * `str_source`: A slice of `String`s representing the original source code lines.
    /// * `push`: The `String` to be inserted.
    /// * `push_where`: A boolean flag; `true` inserts at the beginning, `false` inserts at the end.
    ///
    /// # Returns
    ///
    /// A `Result` containing the modified `Vec<String>`, or an `ErrorHandling` if the `str_source` is empty (preventing whitespace determination) or other unexpected errors occur during string manipulation.
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
    //Splits the string that is usually parsed from fs::read_to_string
    //split_inclusive method is necessary for preserving newline indentation.
    fn string_to_vector(str_source: &str) -> Vec<String> {
        str_source.lines().map(|line| line.to_string()).collect()
    }

    //Main entry for seeker and extract_by_line, roams through Vec<ObjectRange> seeking for the object that fits
    //the requested line number. If it finds no match, then LineOutOfBounds error is thrown
    /// Exports a specific Rust code object from a given source string based on its line number.
    /// It iterates through a slice of `ObjectRange` structs (representing parsed items) to find the object that encompasses the `line_number`.
    /// Once found, it extracts the relevant lines from the source string vector and returns them as a single string.
    ///
    /// # Arguments
    ///
    /// * `line_number`: The 1-based line number used to identify the target object.
    /// * `visited`: A slice of `ObjectRange` structs representing the parsed items within the source.
    /// * `src`: A slice of `String`s representing the lines of the full source code.
    ///
    /// # Returns
    ///
    /// A `Result` containing the extracted object's code as a `String`, or an `ErrorHandling` if no object is found at the specified line or an error occurs during extraction.
    fn export_object(
        line_number: usize,
        visited: &[ObjectRange],
        src: &[String],
    ) -> Result<String, ErrorHandling> {
        for item in visited {
            let found = seeker(line_number, item, src);
            if found.is_err() {
                continue;
            }
            return found;
        }
        Err(ErrorHandling::ExportObjectFailed {
            line_number,
            src: format!("{visited:?}"),
        })
    }

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
//Finds an object, justifying whether the said line number belongs to the range of the object.
//If it does, then object is printed with extract_by_line

fn seeker(line_number: usize, item: &ObjectRange, src: &[String]) -> Result<String, ErrorHandling> {
    let line_start = item.line_ranges.start;
    let line_end = item.line_ranges.end;
    ensure!(
        line_start <= line_number && line_end >= line_number,
        SeekerFailedSnafu { line_number }
    );
    Ok(extract_by_line(src, &line_start, &line_end))
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
