//!Reminder how to use rust_parser:
//! If you want to see the list of objects in a .rs file you have to call parse_all_rust_items
//! Most of the operations revolve around it, as it greps all the object types, their and where they are located
//! This can be easily used via the interface of ObjectRange, which implements 4 functions that are only useful
//! for interacting with it. Hence, with this information about objects, they can exclusively pulled out using the
//! string_to_vec method if you preemptively have taken a list of files that include rust code and have read them into a
//! string type variable.
use snafu::ResultExt;
use snafu::Snafu;
use snafu::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{Item, parse_file};

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
        line_index: usize,
    },
    InvalidIoOperations {
        source: std::io::Error,
    },
    InvalidSynParsing {
        source: syn::Error,
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

pub fn parse_all_rust_items(src: &String) -> Result<Vec<ObjectRange>, ErrorHandling> {
    //Depends on visit_items and find_module_file
    // let src = fs::read_to_string(path).context(InvalidIoOperationsSnafu)?;
    let ast = parse_file(src).context(InvalidSynParsingSnafu)?;
    Ok(visit_items(&ast.items))
}

fn visit_items(items: &[Item]) -> Vec<ObjectRange> {
    let mut object_line: Vec<ObjectRange> = Vec::new();
    for item in items {
        match item {
            Item::Struct(s) => {
                let line_start = s.span().start().line;
                let line_end = s.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("struct"), Name::Name(s.ident.to_string())],
                });
            }
            Item::Enum(e) => {
                let line_start = e.span().start().line;
                let line_end = e.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("enum"), Name::Name(e.ident.to_string())],
                });
            }
            Item::Fn(f) => {
                let line_start = f.span().start().line;
                let line_end = f.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("fn"), Name::Name(f.sig.ident.to_string())],
                });
            }

            Item::Mod(m) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(m.span().start().line),
                        LineRange::End(m.span().end().line),
                    ],
                    names: vec![Name::TypeName("mod"), Name::Name(m.ident.to_string())],
                });
            }

            Item::Use(u) => {
                if let syn::UseTree::Path(path) = u.tree.to_owned() {
                    let path_name = path.ident.to_string();
                    let start = path.span().start().line;
                    let end = path.span().end().line;
                    object_line.push(ObjectRange {
                        line_ranges: vec![LineRange::Start(start), LineRange::End(end)],
                        names: vec![Name::TypeName("use"), Name::Name(path_name)],
                    });
                }
            }

            Item::Impl(i) => {
                let line_start = i.span().start().line;
                let line_end = i.span().end().line;
                let trait_name = match &i.trait_ {
                    Some(trait_) => {
                        let trait_name = trait_.1.get_ident().unwrap().to_string();
                        trait_name
                    }
                    None => "matches struct".to_string(),
                };
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("impl"), Name::Name(trait_name.clone())],
                });
            }
            Item::Trait(t) => {
                let line_start = t.span().start().line;
                let line_end = t.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("trait"), Name::Name(t.ident.to_string())],
                });
            }
            Item::Type(t) => {
                let line_start = t.span().start().line;
                let line_end = t.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("type"), Name::Name(t.ident.to_string())],
                });
            }
            Item::Union(u) => {
                let line_start = u.span().start().line;
                let line_end = u.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("union"), Name::Name(u.ident.to_string())],
                });
            }
            Item::Const(c) => {
                let line_start = c.span().start().line;
                let line_end = c.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("const"), Name::Name(c.ident.to_string())],
                });
            }
            Item::Macro(m) => {
                let macro_name = format!("{:?}", m.mac.clone());
                let line_start = m.span().start().line;
                let line_end = m.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("macro"), Name::Name(macro_name)],
                });
            }

            Item::ExternCrate(c) => {
                let line_start = c.span().start().line;
                let line_end = c.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![
                        Name::TypeName("extern crate"),
                        Name::Name(c.ident.to_string()),
                    ],
                });
            }

            Item::Static(s) => {
                let line_start = s.span().start().line;
                let line_end = s.span().end().line;
                object_line.push(ObjectRange {
                    line_ranges: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    names: vec![Name::TypeName("static"), Name::Name(s.ident.to_string())],
                });
            }
            _ => println!("Other item"),
        }
    }
    object_line
}

pub fn find_module_file(
    base_path: &Path,
    mod_name: &str,
) -> Result<Option<PathBuf>, ErrorHandling> {
    let paths = [
        base_path.join(format!("{}.rs", mod_name)), // mod.rs style
        base_path.join(mod_name).join("mod.rs"),    // mod.rs in subdirectory
    ];

    for path in &paths {
        if path.exists() {
            return Ok(Some(path.to_path_buf()));
        }
    }

    Ok(None)
}

pub fn file_to_vector(file: &Path) -> Result<Vec<String>, ErrorHandling> {
    //Simplified version, using the standard library; functions virtually the same
    let code = fs::read_to_string(file);
    code.map(|code| {
        code.lines()
            .map(|line| line.into())
            .collect::<Vec<String>>()
    })
    .context(InvalidIoOperationsSnafu)
}
pub fn string_to_vector(str_source: String) -> Vec<String> {
    str_source.lines().map(|line| line.to_string()).collect()
}

pub fn receive_context(line_from: usize, str_source: Vec<String>) -> Result<String, ErrorHandling> {
    //let src_path = fs::read_to_string(file_path).context(InvalidIoOperationsSnafu)?;
    let visited = parse_all_rust_items(&str_source.join(""));
    let visited = match visited {
        Ok(visited) => visited,
        Err(_) => {
            return Err(ErrorHandling::ErrorParsingFile {
                in_line: line_from,
                from: str_source.join(""),
            });
        }
    };
    for item in visited {
        let found = seeker(line_from, item, str_source.clone());
        if found.is_err() {
            continue;
        }
        return found;
    }
    Err(ErrorHandling::LineOutOfBounds {
        line_index: line_from,
    })
}
//Extracts a snippet from a file in regard to the snippet boundaries
pub fn extract_by_line(
    from: Vec<String>,
    line_start: &usize,
    line_end: &usize,
) -> Result<String, ErrorHandling> {
    //let vector_of_file = file_to_vector(from)?;
    let line_start = line_start - 1;
    let f = &from[line_start..*line_end];
    Ok(f.join(""))
}

//TESTS FOR SEEKER;
//line_number not line_index
fn seeker(
    line_index: usize,
    item: ObjectRange,
    str_source: Vec<String>,
) -> Result<String, ErrorHandling> {
    let line_start = item.line_start().unwrap();
    let line_end = item.line_end().unwrap();
    ensure!(
        line_start <= line_index && line_end >= line_index,
        LineOutOfBoundsSnafu { line_index }
    );
    extract_by_line(str_source, &line_start, &line_end)
}
