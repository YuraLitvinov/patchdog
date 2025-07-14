//! If you want to see the list of objects in a .rs file you have to call parse_all_rust_items
//! Most of the operations revolve around it, as it greps all the object types, their names, line numbers and where they are located
//! This can be easily used via the interface of ObjectRange, which implements 4 functions that are only useful
//! for interacting with it. Hence, with this information about objects, they can exclusively pulled out using the
//! string_to_vec method if you preemptively have taken a list of files that include rust code and have read them into a
//! string type variable.
//! Syn crate itself provides functionality to pull out objects from a file, albeit it loses very helpful //comments, so instead
//! it was chosen as best practice to only get line numbers and from there pull out the whole object.
//! Error handling is carried out with SNAFU. 
pub mod error;
pub mod file_parsing;
pub mod object_range;
pub mod rust_parser;
pub mod rustc_parsing;

pub use error::ErrorHandling;
pub use object_range::ObjectRange;
pub use rustc_parsing::comment_lexer;
