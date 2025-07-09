//! If you want to see the list of objects in a .rs file you have to call parse_all_rust_items
//! Most of the operations revolve around it, as it greps all the object types, their names, line numbers and where they are located
//! This can be easily used via the interface of ObjectRange, which implements 4 functions that are only useful
//! for interacting with it. Hence, with this information about objects, they can exclusively pulled out using the
//! string_to_vec method if you preemptively have taken a list of files that include rust code and have read them into a
//! string type variable.
//! Syn crate itself provides functionality to pull out objects from a file, albeit it loses very helpful //comments, so instead
//! it was chosen as best practice to only get line numbers and from there pull out the whole object.
use snafu::ResultExt;
use snafu::Snafu;
use snafu::prelude::*;
use std::path::PathBuf;
use syn::spanned::Spanned;
use syn::{Item, parse_str};
pub mod rustc_parsing;

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

//Wrapper for visit_items that handles errors and outputs result of visit_items for a file
pub fn parse_all_rust_items(src: String) -> Result<Vec<ObjectRange>, ErrorHandling> {
    //Depends on visit_items and find_module_file
    let ast: syn::File = parse_str(&src).context(InvalidSynParsingSnafu)?; //Actually, parses any string, that would contain valid rust code
    Ok(visit_items(&ast.items))
}
//This structure is static. It finds matches within the file that are rust objects.
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

            Item::Mod(m) => match m.content.clone() {
                Some((_, items)) => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.ident.span().start().line),
                            LineRange::End(m.ident.span().end().line),
                        ],
                        names: vec![Name::TypeName("mod"), Name::Name(m.ident.to_string())],
                    });
                    object_line.extend(visit_items(&items));
                }
                None => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.ident.span().start().line),
                            LineRange::End(m.ident.span().end().line),
                        ],
                        names: vec![Name::TypeName("mod"), Name::Name(m.ident.to_string())],
                    });
                }
            },

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
//Roams through lib.rs file seeking for mod objects that are indicators for files within the same folder as i.e. lib.rs
//Is used to recursively parse all objects in said file
pub fn find_module_file(
    base_path: PathBuf,
    mod_name: String,
) -> Result<Option<PathBuf>, ErrorHandling> {
    let mut path = base_path;
    path.pop();
    let paths = [path.join(format!("{}.rs", mod_name))];
    for path in paths {
        if path.exists() {
            return Ok(Some(path.to_path_buf()));
        }
    }

    Ok(None)
}
//Splits the string that is usually parsed from fs::read_to_string
//split_inclusive method is necessary for preserving newline indentation.
pub fn string_to_vector(str_source: &str) -> Vec<String> {
    str_source
        .split_inclusive('\n')
        .map(|line| line.to_string())
        .collect()
}
//Main entry for seeker and extract_by_line, roams through Vec<ObjectRange> seeking for the object that fits
//the requested line number. If it finds no match, then LineOutOfBounds error is thrown
pub fn export_object(
    from_line_number: usize,
    visited: Vec<ObjectRange>,
    src: &Vec<String>,
) -> Result<String, ErrorHandling> {
    for item in &visited {
        let found = seeker(from_line_number, item, src);
        if found.is_err() {
            continue;
        }
        return found;
    }
    Err(ErrorHandling::ExportObjectFailed {
        line_number: from_line_number,
        src: src[from_line_number].clone(),
    })
}
//Finds an object, justifying whether the said line number belongs to the range of the object.
//If it does, then object is printed with extract_by_line
pub fn seeker(
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
    Ok(extract_by_line(src, &line_start, &line_end))
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
    Ok(extract_by_line(&src, &line_start, &line_end))
}
//Extracts a snippet from a file in regard to the snippet boundaries
pub fn extract_by_line(from: &[String], line_start: &usize, line_end: &usize) -> String {
    let line_start = line_start - 1;

    from[line_start..*line_end].join("")
}
pub fn extract_object_preserving_comments(
    src: Vec<String>,
    from_line: usize,
    parsed: Vec<ObjectRange>,
) -> Result<String, ErrorHandling> {
    let mut new_previous: Vec<usize> = Vec::new();
    new_previous.push(1);
    let mut i = 0;
    for each in parsed {
        //println!("{} {}", new_previous[i], each.line_end().unwrap());
        let found = seeker_for_comments(
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
        let extracted = extract_by_line(
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
