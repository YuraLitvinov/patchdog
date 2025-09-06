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
impl ObjectRange {
/// Retrieves the name of the Rust object represented by the `ObjectRange` instance.
/// This method provides a direct way to access the human-readable identifier of the parsed Rust item, such as a function name, struct name, or enum variant name.
/// The name is extracted from the `names` field within the `ObjectRange`.
///
/// # Arguments
///
/// * `&self` - A reference to the `ObjectRange` instance.
///
/// # Returns
///
/// A `String` representing the name of the object.
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
/// Retrieves the type descriptor of the Rust object represented by the `ObjectRange` instance.
/// This method provides a clear indication of what kind of Rust item the `ObjectRange` represents, such as "fn" for a function, "struct" for a structure, or "enum" for an enumeration.
/// The type name is extracted from the `names` field within the `ObjectRange`.
///
/// # Arguments
///
/// * `&self` - A reference to the `ObjectRange` instance.
///
/// # Returns
///
/// A `String` representing the type of the object.
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
/// Retrieves the 1-based starting line number of the Rust object represented by the `ObjectRange` instance.
/// This method offers a convenient way to determine where a particular Rust item begins in the source file.
/// The starting line number is directly accessed from the `line_ranges` field of the `ObjectRange`.
///
/// # Arguments
///
/// * `&self` - A reference to the `ObjectRange` instance.
///
/// # Returns
///
/// A `usize` representing the 1-based starting line number of the object.
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
/// Retrieves the 1-based ending line number of the Rust object represented by the `ObjectRange` instance.
/// This method provides a straightforward way to determine the last line where a particular Rust item's definition concludes in the source file.
/// The ending line number is directly accessed from the `line_ranges` field of the `ObjectRange`.
///
/// # Arguments
///
/// * `&self` - A reference to the `ObjectRange` instance.
///
/// # Returns
///
/// A `usize` representing the 1-based ending line number of the object.
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
