use crate::{ErrorHandling, LineOutOfBoundsSnafu, ObjectRange, SeekerFailedSnafu};
use snafu::ensure;
pub struct FileExtractor;
pub trait Files {
    fn check_for_not_comment(
        parsed: &Vec<ObjectRange>,
        line_number: usize,
    ) -> Result<bool, ErrorHandling>;
    fn extract_object_preserving_comments(
        src: Vec<String>,
        from_line: usize,
        parsed: Vec<ObjectRange>,
    ) -> Result<String, ErrorHandling>;
    fn extract_by_line(from: &[String], line_start: &usize, line_end: &usize) -> String;
    fn seeker_for_comments(
        line_number: usize,
        line_start: usize,
        line_end: usize,
        src: Vec<String>,
    ) -> Result<String, ErrorHandling>;
    fn seeker(
        line_number: usize,
        item: &ObjectRange,
        src: &Vec<String>,
    ) -> Result<String, ErrorHandling>;
    fn export_object(
        from_line_number: usize,
        visited: &Vec<ObjectRange>,
        src: &Vec<String>,
    ) -> Result<String, ErrorHandling>;
    fn string_to_vector(str_source: &str) -> Vec<String>;
    fn return_match(
        line_number: usize,
        visited: Vec<ObjectRange>,
    ) -> Result<ObjectRange, ErrorHandling>;
    fn return_sought(line_number: usize, item: ObjectRange) -> Result<ObjectRange, ErrorHandling>;
}

impl Files for FileExtractor {
    //Splits the string that is usually parsed from fs::read_to_string
    //split_inclusive method is necessary for preserving newline indentation.
    fn string_to_vector(str_source: &str) -> Vec<String> {
        str_source.lines().map(|line| line.to_string()).collect()
    }
    //Main entry for seeker and extract_by_line, roams through Vec<ObjectRange> seeking for the object that fits
    //the requested line number. If it finds no match, then LineOutOfBounds error is thrown
    fn export_object(
        line_number: usize,
        visited: &Vec<ObjectRange>,
        src: &Vec<String>,
    ) -> Result<String, ErrorHandling> {
        for item in visited {
            let found = Self::seeker(line_number, item, src);
            if found.is_err() {
                continue;
            }
            return found;
        }
        Err(ErrorHandling::ExportObjectFailed {
            line_number: line_number,
            src: src[line_number].clone(),
        })
    }
    fn return_match(
        line_number: usize,
        visited: Vec<ObjectRange>,
    ) -> Result<ObjectRange, ErrorHandling> {
        for item in visited {
            let found = Self::return_sought(line_number, item);
            if found.is_err() {
                continue;
            }
            return found;
        }
        Err(ErrorHandling::ExportObjectFailed {
            line_number: line_number,
            src: "refers to return_match".to_string(),
        })
    }
    fn return_sought(line_number: usize, item: ObjectRange) -> Result<ObjectRange, ErrorHandling> {
        let line_start = item.line_start().unwrap();
        let line_end = item.line_end().unwrap();
        ensure!(
            line_start <= line_number && line_end >= line_number,
            SeekerFailedSnafu { line_number }
        );
        Ok(item)
    }
    //Finds an object, justifying whether the said line number belongs to the range of the object.
    //If it does, then object is printed with extract_by_line
    fn seeker(
        line_number: usize,
        item: &ObjectRange,
        src: &Vec<String>,
    ) -> Result<String, ErrorHandling> {
        let line_start = item.line_start().unwrap();
        let line_end = item.line_end().unwrap();
        ensure!(
            line_start <= line_number && line_end >= line_number,
            SeekerFailedSnafu { line_number }
        );
        Ok(Self::extract_by_line(src, &line_start, &line_end))
    }
    fn seeker_for_comments(
        line_number: usize,
        line_start: usize,
        line_end: usize,
        src: Vec<String>,
    ) -> Result<String, ErrorHandling> {
        ensure!(
            line_start <= line_number && line_end >= line_number,
            LineOutOfBoundsSnafu { line_number }
        );
        Ok(Self::extract_by_line(&src, &line_start, &line_end))
    }
    //Extracts a snippet from a file in regard to the snippet boundaries
    fn extract_by_line(from: &[String], line_start: &usize, line_end: &usize) -> String {
        let line_start = line_start - 1;

        from[line_start..*line_end].join("\n")
    }
    fn extract_object_preserving_comments(
        src: Vec<String>,
        from_line: usize,
        parsed: Vec<ObjectRange>,
    ) -> Result<String, ErrorHandling> {
        let mut new_previous: Vec<usize> = Vec::new();
        new_previous.push(1);
        let mut i = 0;
        for each in parsed {
            //println!("{} {}", new_previous[i], each.line_end().unwrap());
            let found = Self::seeker_for_comments(
                from_line,
                new_previous[i],
                each.line_end().unwrap(),
                src.clone(),
            );
            if found.is_err() {
                i += 1;
                let previous_end_line = each
                    .line_end()
                    .expect("Failed to unwrap ObjectRange for line end")
                    + 1;
                new_previous.push(previous_end_line);
                continue;
            }
            let extracted = Self::extract_by_line(
                &src,
                &new_previous[i],
                &each
                    .line_end()
                    .expect("Failed to unwrap ObjectRange for line end"),
            );
            return Ok(extracted);
        }
        Err(ErrorHandling::LineOutOfBounds { line_number: 0 })
    }
    fn check_for_not_comment(
        parsed: &Vec<ObjectRange>,
        line_number: usize,
    ) -> Result<bool, ErrorHandling> {
        for each in parsed {
            if each.line_start().unwrap() <= line_number && line_number <= each.line_end().unwrap()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
