use snafu::{ResultExt, Whatever};
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{Item, parse_file};
#[derive(Debug)]
enum LineRange {
    Start(usize),
    End(usize),
}
#[derive(Debug)]
enum Name {
    TypeName(&'static str),
    Name(String),
}
#[derive(Debug)]
pub struct ObjectRange { //There is an ample interface for interaction with this structure, hence, I believe there is no reason to change it
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

pub fn frontend_visit_items(item: &ObjectRange) -> Vec<&ObjectRange> {
    vec![item]
}

pub fn parse_all_rust_items(path: &Path) -> Result<Vec<ObjectRange>, Whatever> {
    //Depends on visit_items and find_module_file
    let src = fs::read_to_string(path)
        .with_whatever_context(|_| format!("Failed to read file: {path:?}"));
    //println!("{:?}", &path);
    let ast_src = match src {
        Ok(src) => src,
        Err(why) => {
            eprintln!("{}", why);
            return Err(why);
        }
    };
    let ast = match parse_file(&ast_src).with_whatever_context(|_| {
        format!("Failed to parse file: {path:?} \n Does it contain Rust code?")
    }) {
        Ok(ast) => ast,
        Err(why) => {
            eprintln!("{}", why);
            return Err(why);
        }
    };
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
                /*
                if let Some((_, items)) = &m.content {
                    // Inline module
                    visit_items(items, base_path);
                } else {
                    // External module: look for file on disk
                    let mod_path = find_module_file(base_path, &m.ident.to_string());
                    if let Some(mod_file) = mod_path {
                        parse_all_rust_items(&mod_file);
                    }
                }
                */
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
                    names: vec![Name::TypeName("impl"), Name::Name(trait_name)],
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
                let macro_name = format!("{:?}", m.ident.clone().unwrap());
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

pub fn find_module_file(base_path: &Path, mod_name: &str) -> Result<Option<PathBuf>, Whatever> {
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

pub fn file_to_vector(file: &Path) -> Result<Vec<String>, Whatever> {
    //Simplified version, using the standard library; functions virtually the same
    let code = fs::read_to_string(file)
        .with_whatever_context(|_| format!("Failed to read file: {file:?}"));
    let collected_vector = match code {
        Ok(code) => Ok(code.lines().map(|line| line.to_string()).collect()),
        Err(why) => Err(why),
    };
    collected_vector
}

pub fn extract_function(
    from: &Path,
    line_start: &usize,
    line_end: &usize,
) -> Result<String, Whatever> {
    let vector_of_file = file_to_vector(from)?;
    let line_start = line_start - 1;
    let f = &vector_of_file[line_start..*line_end].join("\n");
    //parse_all_rust_items(std::path::Path::new(f));
    //println!("{}", f);
    Ok(f.to_string())
}
