use std::ops::Range;

use serde::Serialize;

#[derive(Debug, Clone, serde::Deserialize, Serialize, PartialEq, Hash, Eq)]
pub struct Name {
    pub type_name: String,
    pub name: String,
}
#[derive(Debug, Clone, serde::Deserialize, Serialize, PartialEq, Hash, Eq)]
pub struct ObjectRange {
    //There is an ample interface for interaction with this structure, hence, I believe there is no reason to change it
    pub line_ranges: Range<usize>, // Has to stay, as a lot of functionality is bound to this field
    pub names: Name,
}
/*
Calling each object with ObjectRange
object_name = %object%.object_name()
object_type = %object%.object_type()
line_start = %object%.line_start()
line_end = %object%.line_end()
*/

impl Default for ObjectRange {
/// Implements the `Default` trait, providing a default constructor for the struct.
/// This method initializes the `line_ranges` field to an empty range (`0..0`) and the `names` field with empty strings for both `type_name` and `name`. It is useful for creating a base instance of the struct when no specific initial values are provided.
///
/// # Returns
/// A new instance of `Self` with all fields initialized to their default values.
    fn default() -> Self {
        Self {
            line_ranges: 0..0,
            names: Name {
                type_name: "".to_string(),
                name: "".to_string(),
            },
        }
    }
}
impl ObjectRange {

/// Retrieves the name of the code object from the `names` property of the `ObjectRange` struct.
/// This method provides direct access to the `name` field, which typically represents the identifier (e.g., function name, struct name) of a parsed Rust code item. It returns the name as a `String`.
///
/// # Returns
/// A `String` representing the name of the object.
/// Returns the name of the code object. This method provides direct access to the `name` field within the `names` property of the `ObjectRange` struct.
/// It is primarily used to retrieve the identifier (e.g., function name, struct name) associated with a parsed Rust code item.
///
/// # Returns
///
/// A `String` representing the name of the object.
    pub fn object_name(&self) -> String {
        self.names.name.to_string()
    }

/// Returns the type name of the code object. This method provides a convenient way to access the `type_name` field stored within the `names` property of the `ObjectRange` struct.
/// It is useful for quickly identifying whether an object represents a function, struct, enum, or other Rust item without direct access to the `names` field.
///
/// # Returns
///
/// A `String` representing the type name of the object (e.g., "fn", "struct", "enum").
    pub fn object_type(&self) -> String {
        self.names.type_name.to_string()
    }

/// Returns the starting line number of the code object's range. This method provides a direct way to access the lower bound (inclusive) of the line range that the `ObjectRange` instance occupies.
/// It is commonly used when needing to locate or highlight the beginning of a parsed code item.
///
/// # Returns
///
/// A `usize` representing the start line number of the object's code block.
    pub fn line_start(&self) -> usize {
        self.line_ranges.start
    }

/// Returns the ending line number of the code object's range. This method provides a simple way to get the upper bound (exclusive) of the line range that the `ObjectRange` instance encompasses.
/// It is often used in conjunction with `line_start` to define the full extent of a code item within a file.
///
/// # Returns
///
/// A `usize` representing the end line number of the object's code block.
    pub fn line_end(&self) -> usize {
        self.line_ranges.end
    }
}
