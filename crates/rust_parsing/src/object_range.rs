use std::ops::Range;

use serde::Serialize;

#[derive(Debug, Clone, serde::Deserialize, Serialize)]
pub struct Name {
    pub type_name: String,
    pub name: String,
}
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
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
    pub fn object_name(&self) -> String {
        self.names.name.to_string()
    }
    pub fn object_type(&self) -> String {
        self.names.type_name.to_string()
    }
    pub fn line_start(&self) -> usize {
        self.line_ranges.start
    }
    pub fn line_end(&self) -> usize {
        self.line_ranges.end
    }
}
