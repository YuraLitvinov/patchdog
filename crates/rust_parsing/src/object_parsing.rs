
use crate::{ObjectRange, ErrorHandling, LineRange, Name, InvalidSynParsingSnafu};
use syn::parse_str;
use syn::Item;
use snafu::ResultExt;
use syn::spanned::Spanned;
use std::path::PathBuf;
pub fn parse_all_rust_items(src: String) -> Result<Vec<ObjectRange>, ErrorHandling> {
    //Depends on visit_items and find_module_file
    let ast: syn::File = parse_str(&src).context(InvalidSynParsingSnafu)?; //Actually, parses any string, that would contain valid rust code
    Ok(visit_items(&ast.items))
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
                            LineRange::Start(m.span().start().line),
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
