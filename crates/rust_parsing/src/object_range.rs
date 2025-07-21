#[derive(Debug)]
pub enum LineRange {
    Start(usize),
    End(usize),
}
#[derive(Debug)]
pub enum Name {
    TypeName(String),
    Name(String),
}
#[derive(Debug)]
pub struct ObjectRange {
    //There is an ample interface for interaction with this structure, hence, I believe there is no reason to change it
    pub(crate) line_ranges: Vec<LineRange>, // Has to stay, as a lot of functionality is bound to this field
    pub(crate) names: Vec<Name>,
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
