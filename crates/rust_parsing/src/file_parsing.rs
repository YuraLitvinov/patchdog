use crate::error::{ErrorHandling, InvalidIoOperationsSnafu};
use crate::object_range::ObjectRange;
use crate::rust_parser::RustItemParser;
use crate::rust_parser::RustParser;
use snafu::ResultExt;
use tracing::Level;
use std::{fs::File, io::Write, path::PathBuf};
pub const REGEX: &str = r#"\{\s*"uuid"\s*:\s*"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",\s*"new_comment"\s*:\s*".*"\s*\}"#;
pub struct FileExtractor;
pub trait Files {
    fn check_for_valid_object(
        parsed: &[ObjectRange],
        line_number: usize,
    ) -> Result<bool, ErrorHandling>;

    fn string_to_vector(source: &str) -> Vec<String>;

    //Representing file as a vector of lines is generally the most effective practise
    fn write_to_vecstring(
        path: PathBuf,
        source: Vec<String>,
        line_index: usize,
        changed_element: String,
    ) -> Result<(), ErrorHandling>;
}

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


pub fn write_with_remove(path: PathBuf, source: Vec<String>, line_index: usize, changes: String) -> Result<(), ErrorHandling> {
    
    let source_text = source.join("\n");

    let parsed = RustItemParser::parse_all_rust_items(&source_text).expect("Failed to parse");
    let functions = parsed.clone()
        .into_iter()
        .filter_map(|each| {
            if each.names.type_name == "fn" {
                Some(each)
            } else {
                None
            }
        }).collect::<Vec<ObjectRange>>();
    let function_at_line_index = functions
        .iter()
        .find(|each| {
            !FileExtractor::check_for_valid_object(&[(*each).clone()], line_index)
                .expect("Failed to check")
        });
    if let Some(val) = function_at_line_index {
        //Gatekeep against functions with no prior documentation
        let comments = parsed.clone()
            .into_iter()
            .filter_map(|each|
                if each.names.name == "///" {
                    Some(each)
                } else {
                    None
                }
            )
            .collect::<Vec<ObjectRange>>();
        if comments.is_empty() {
            tracing::event!(Level::INFO, "{}", comments.len());
            FileExtractor::write_to_vecstring(path, source, line_index, changes)?;

            return Ok(());
        }
        let remove_from_original = FileExtractor::string_to_vector(&source_text);
        let mut remove_from_original = remove_from_original.clone();
    
        let collect_all_comment_in_range_of_function = comments.clone()
            .into_iter()
            .filter_map(|each| {
                if each.line_ranges.start >= val.line_ranges.start && each.line_ranges.end <= val.line_ranges.end        {
                    Some(each)
                } else {
                    None
                }
            })
            .collect::<Vec<ObjectRange>>();
        if let Some(first_value) = collect_all_comment_in_range_of_function.first() && 
            let Some(last_value) = collect_all_comment_in_range_of_function.last() {
                remove_from_original.drain(first_value.line_start()-1..last_value.line_end());
                let actual_function_object = ObjectRange {
                    line_ranges: last_value.line_end()+1..val.line_end(),
                    names: val.names.clone(),
                };
                tracing::event!(Level::INFO, "{function_at_line_index:?}");
            
                tracing::event!(Level::INFO, "{actual_function_object:?}");
            
                let no_comment = source[actual_function_object.line_start()-1..actual_function_object.line_end()].join("\n");
                let mut clone_source = source.clone();
                clone_source.insert(val.line_end(), no_comment);
                clone_source.drain(val.line_start()-1..actual_function_object.line_end());
            
                FileExtractor::write_to_vecstring(path, clone_source, line_index, changes)?;
            
                return Ok(());     
            }

    }
    
    Ok(())
}