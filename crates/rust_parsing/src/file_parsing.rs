use crate::error::{CouldNotGetLineSnafu, ErrorHandling, LineOutOfBoundsSnafu, SeekerFailedSnafu};
use crate::object_range::ObjectRange;
use snafu::{OptionExt, ensure};
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
}

impl Files for FileExtractor {
    fn check_for_valid_object(
        parsed: &[ObjectRange],
        line_number: usize,
    ) -> Result<bool, ErrorHandling> {
        for each in parsed {
            if each.line_start().context(CouldNotGetLineSnafu)? <= line_number
                && line_number <= each.line_end().context(CouldNotGetLineSnafu)?
            {
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
            src: format!("{:?}", visited),
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
            let found = seeker_for_comments(
                from_line,
                new_previous[i],
                each.line_end().context(CouldNotGetLineSnafu)?,
                &src,
            );
            if found.is_err() {
                i += 1;
                let previous_end_line = each.line_end().context(CouldNotGetLineSnafu)? + 1;
                new_previous.push(previous_end_line);
                continue;
            }
            let extracted = extract_by_line(
                &src,
                &new_previous[i],
                &each.line_end().context(CouldNotGetLineSnafu)?,
            );
            return Ok(extracted);
        }
        Err(ErrorHandling::LineOutOfBounds { line_number: 0 })
    }
}
//Finds an object, justifying whether the said line number belongs to the range of the object.
//If it does, then object is printed with extract_by_line


fn seeker(line_number: usize, item: &ObjectRange, src: &[String]) -> Result<String, ErrorHandling> {
    let line_start = item.line_start().context(CouldNotGetLineSnafu)?;
    let line_end = item.line_end().context(CouldNotGetLineSnafu)?;
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
    src: &Vec<String>,
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
