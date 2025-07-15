use crate::error::{ErrorHandling, InvalidSynParsingSnafu};
use crate::object_range::{LineRange, Name, ObjectRange};
use snafu::ResultExt;
use std::path::PathBuf;
use syn::File;
use syn::parse_str;
use syn::spanned::Spanned;
use syn::{ImplItem, Item};

pub trait RustParser {
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn find_module_file(
        base_path: PathBuf,
        mod_name: String,
    ) -> Result<Option<PathBuf>, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let ast: File = parse_str(src).context(InvalidSynParsingSnafu)?;
        Ok(visit_items(&ast.items))
    }

    fn find_module_file(
        base_path: PathBuf,
        mod_name: String,
    ) -> Result<Option<PathBuf>, ErrorHandling> {
        let mut path = base_path;
        path.pop();
        let paths = [path.join(format!("{}.rs", mod_name))];
        for path in paths {
            if path.exists() {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
}

fn visit_items(items: &[Item]) -> Vec<ObjectRange> {
    let mut object_line: Vec<ObjectRange> = Vec::new();

    for item in items {
        match item {
            Item::Struct(s) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(s.span().start().line),
                        LineRange::End(s.span().end().line),
                    ],
                    names: vec![Name::TypeName("struct"), Name::Name(s.ident.to_string())],
                });
            }
            Item::Enum(e) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(e.span().start().line),
                        LineRange::End(e.span().end().line),
                    ],
                    names: vec![Name::TypeName("enum"), Name::Name(e.ident.to_string())],
                });
            }
            Item::Fn(f) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(f.span().start().line),
                        LineRange::End(f.span().end().line),
                    ],
                    names: vec![Name::TypeName("fn"), Name::Name(f.sig.ident.to_string())],
                });
            }
            Item::Mod(m) => match &m.content {
                Some((_, items)) => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.span().start().line),
                            LineRange::End(m.ident.span().end().line),
                        ],
                        names: vec![Name::TypeName("mod"), Name::Name(m.ident.to_string())],
                    });
                    object_line.extend(visit_items(items));
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
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(path.span().start().line),
                            LineRange::End(path.span().end().line),
                        ],
                        names: vec![Name::TypeName("use"), Name::Name(path.ident.to_string())],
                    });
                }
            }
            Item::Impl(i) => {
                let trait_name = match &i.trait_ {
                    Some((_, path, _)) => path
                        .segments
                        .last()
                        .expect("failed to get impl name")
                        .ident
                        .to_string(),
                    None => "matches struct".to_string(),
                };
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(i.span().start().line),
                        LineRange::End(i.span().end().line),
                    ],
                    names: vec![Name::TypeName("impl"), Name::Name(trait_name)],
                });
                for each_block in &i.items {
                    match each_block {
                        ImplItem::Fn(f) => {
                            object_line.push(ObjectRange {
                                line_ranges: vec![
                                    LineRange::Start(f.span().start().line),
                                    LineRange::End(f.span().end().line),
                                ],
                                names: vec![
                                    Name::TypeName("fn"),
                                    Name::Name(f.sig.ident.to_string()),
                                ],
                            });
                        }
                        ImplItem::Const(c) => {
                            object_line.push(ObjectRange {
                                line_ranges: vec![
                                    LineRange::Start(c.span().start().line),
                                    LineRange::End(c.span().end().line),
                                ],
                                names: vec![
                                    Name::TypeName("const"),
                                    Name::Name(c.ident.to_string()),
                                ],
                            });
                        }
                        ImplItem::Type(t) => {
                            object_line.push(ObjectRange {
                                line_ranges: vec![
                                    LineRange::Start(t.span().start().line),
                                    LineRange::End(t.span().end().line),
                                ],
                                names: vec![
                                    Name::TypeName("type"),
                                    Name::Name(t.ident.to_string()),
                                ],
                            });
                        }
                        ImplItem::Macro(m) => {
                            object_line.push(ObjectRange {
                                line_ranges: vec![
                                    LineRange::Start(m.span().start().line),
                                    LineRange::End(m.span().end().line),
                                ],
                                names: vec![
                                    Name::TypeName("macro"),
                                    Name::Name(format!("{:?}", m.mac.path)),
                                ],
                            });
                        }
                        ImplItem::Verbatim(v) => {
                            object_line.push(ObjectRange {
                                line_ranges: vec![
                                    LineRange::Start(v.span().start().line),
                                    LineRange::End(v.span().end().line),
                                ],
                                names: vec![Name::TypeName("verbatim"), Name::Name(v.to_string())],
                            });
                        }
                        _ => println!("Other impl object"),
                    }
                }
            }
            Item::Trait(t) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(t.span().start().line),
                        LineRange::End(t.span().end().line),
                    ],
                    names: vec![Name::TypeName("trait"), Name::Name(t.ident.to_string())],
                });
            }
            Item::Type(t) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(t.span().start().line),
                        LineRange::End(t.span().end().line),
                    ],
                    names: vec![Name::TypeName("type"), Name::Name(t.ident.to_string())],
                });
            }
            Item::Union(u) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(u.span().start().line),
                        LineRange::End(u.span().end().line),
                    ],
                    names: vec![Name::TypeName("union"), Name::Name(u.ident.to_string())],
                });
            }
            Item::Const(c) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(c.span().start().line),
                        LineRange::End(c.span().end().line),
                    ],
                    names: vec![Name::TypeName("const"), Name::Name(c.ident.to_string())],
                });
            }
            Item::Macro(m) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(m.span().start().line),
                        LineRange::End(m.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("macro"),
                        Name::Name(format!("{:?}", m.mac.path)),
                    ],
                });
            }
            Item::ExternCrate(c) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(c.span().start().line),
                        LineRange::End(c.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("extern crate"),
                        Name::Name(c.ident.to_string()),
                    ],
                });
            }
            Item::Static(s) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(s.span().start().line),
                        LineRange::End(s.span().end().line),
                    ],
                    names: vec![Name::TypeName("static"), Name::Name(s.ident.to_string())],
                });
            }
            _ => println!("Other item"),
        }
    }

    object_line
}
