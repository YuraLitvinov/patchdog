use std::ops::Range;

use serde::Serialize;

#[derive(Debug, Clone, serde::Deserialize, Serialize, PartialEq)]
pub struct Name {
    pub type_name: String,
    pub name: String,
}
#[derive(Debug, Clone, serde::Deserialize, Serialize, PartialEq)]
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
impl ObjectRange {
    /// Retrieves the name of the parsed Rust object.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current `ObjectRange` instance.
    ///
    /// # Returns
    ///
    /// A `String` representing the name of the object (e.g., function name, struct name).
    pub fn object_name(&self) -> String {
        self.names.name.to_string()
    }
    /// Retrieves the type name of the parsed Rust object.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current `ObjectRange` instance.
    ///
    /// # Returns
    ///
    /// A `String` representing the type of the object (e.g., "fn", "struct", "enum").
    pub fn object_type(&self) -> String {
        self.names.type_name.to_string()
    }
    /// Retrieves the starting line number of the parsed Rust object.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current `ObjectRange` instance.
    ///
    /// # Returns
    ///
    /// A `usize` representing the 1-based starting line number of the object.
    pub fn line_start(&self) -> usize {
        self.line_ranges.start
    }
    /// Retrieves the ending line number of the parsed Rust object.
    ///
    /// # Arguments
    ///
    /// * `&self` - A reference to the current `ObjectRange` instance.
    ///
    /// # Returns
    ///
    /// A `usize` representing the 1-based ending line number of the object.
    pub fn line_end(&self) -> usize {
        self.line_ranges.end
    }
}
