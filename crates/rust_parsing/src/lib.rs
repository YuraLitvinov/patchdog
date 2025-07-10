//! If you want to see the list of objects in a .rs file you have to call parse_all_rust_items
//! Most of the operations revolve around it, as it greps all the object types, their names, line numbers and where they are located
//! This can be easily used via the interface of ObjectRange, which implements 4 functions that are only useful
//! for interacting with it. Hence, with this information about objects, they can exclusively pulled out using the
//! string_to_vec method if you preemptively have taken a list of files that include rust code and have read them into a
//! string type variable.
//! Syn crate itself provides functionality to pull out objects from a file, albeit it loses very helpful //comments, so instead
//! it was chosen as best practice to only get line numbers and from there pull out the whole object.
use snafu::Snafu;
pub mod rustc_parsing;
pub mod rust_parser;
pub mod file_parsing;
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ErrorHandling {
    #[snafu(display("Unable to read {line_index} from file {file_name}"))]
    BadFile {
        line_index: usize,
        file_name: String,
    },
    InvalidLineRange {
        line_start: usize,
        line_end: usize,
    },
    ErrorParsingFile {
        in_line: usize,
        from: String,
    },

    LineOutOfBounds {
        line_number: usize,
    },
    InvalidIoOperations {
        source: std::io::Error,
    },
    InvalidSynParsing {
        source: syn::Error,
    },
    SeekerFailed {
        line_number: usize,
    },
    ExportObjectFailed {
        line_number: usize,
        src: String,
    },
}
#[derive(Debug)]
pub enum LineRange {
    Start(usize),
    End(usize),
}
#[derive(Debug)]
pub enum Name {
    TypeName(&'static str),
    Name(String),
}
#[derive(Debug)]
pub struct ObjectRange {
    //There is an ample interface for interaction with this structure, hence, I believe there is no reason to change it
    line_ranges: Vec<LineRange>, // Has to stay, as a lot of functionality is bound to this field
    names: Vec<Name>,
}
/*
Calling each object with ObjectRange
object_name = %object%.object_name().unwrap()
object_type = %object%.object_type().unwrap()
line_start = %object%.line_start().unwrap()
line_end = %object%.line_end().unwrap()
*/
impl ObjectRange {
    pub fn object_name(&self) -> Option<String> {
        for n in &self.names {
            if let Name::Name(val) = n {
                return Some(val.to_string());
            }
        }
        None
    }
    pub fn object_type(&self) -> Option<String> {
        for n in &self.names {
            if let Name::TypeName(val) = n {
                return Some(val.to_string());
            }
        }
        None
    }
    pub fn line_start(&self) -> Option<usize> {
        for r in &self.line_ranges {
            if let LineRange::Start(val) = r {
                return Some(*val);
            }
        }
        None
    }
    pub fn line_end(&self) -> Option<usize> {
        for r in &self.line_ranges {
            if let LineRange::End(val) = r {
                return Some(*val);
            }
        }
        None
    }
}

