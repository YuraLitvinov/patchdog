use crate::error::{ErrorHandling, InvalidIoOperationsSnafu, InvalidItemParsingSnafu};
use crate::file_parsing::{FileExtractor, Files};
use crate::object_range::{LineRange, Name, ObjectRange};
use proc_macro2::TokenStream;
use proc_macro2::{Spacing, TokenTree};
use quote::ToTokens;
use rustc_lexer::TokenKind;
use rustc_lexer::tokenize;
use snafu::ResultExt;
use std::ops::Range;
use std::path::{PathBuf, Path};
use std::{fs, vec};
use syn::spanned::Spanned;
use syn::{AngleBracketedGenericArguments, PathArguments, Type, TypePath};
use syn::{File, ReturnType};
use syn::{FnArg, parse_str};
use syn::{ImplItem, Item};

#[allow(dead_code)]
#[derive(Debug)]
pub struct FunctionSignature {
    fn_input: Vec<FnInputToken>,
    fn_out: FnOutputToken,
}
#[derive(Debug)]
#[allow(dead_code)]
struct FnInputToken {
    input_name: String,
    input_type: String,
}
#[derive(Debug)]
#[allow(dead_code)]
struct FnOutputToken {
    kind: String,
    output_type: String,
    error_type: Option<String>,
}

pub trait RustParser {
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn find_module_file(
        base_path: PathBuf,
        mod_name: String,
    ) -> Result<Option<PathBuf>, ErrorHandling>;
    fn rust_function_parser(src: &str) -> Result<FunctionSignature, ErrorHandling>;
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn rust_ast(src: &str) -> Result<File, ErrorHandling>;
    fn rust_item_parser(src: &str, range: Range<usize>) -> Result<Vec<ObjectRange>, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let file = fs::read_to_string(src).context(InvalidIoOperationsSnafu)?;
        let ast: File = parse_str(&file).context(InvalidItemParsingSnafu { str_source: src })?;
        visit_items(&ast.items)
    }

    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let src_format_error = format!("{:#?}", &src);
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {
            str_source: src_format_error,
        })?;
        let mut comments = comment_lexer(src)?;
        let mut visited = visit_items(&ast.items)?;
        visited.append(&mut comments);
        visited.sort_by_key(|line_obj| {
            line_obj
                .line_ranges
                .iter()
                .filter_map(|linerange| {
                    if let LineRange::Start(n) = linerange {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .min()
                .unwrap_or(usize::MAX)
        });

        Ok(visited)
    }

    fn rust_function_parser(src: &str) -> Result<FunctionSignature, ErrorHandling> {
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {
            str_source: &src.to_string(),
        })?;
        function_parse(&ast.items)
    }

    fn rust_item_parser(src: &str, range: Range<usize>) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let mut visit: Vec<ObjectRange> = Vec::new();
        let src_format_error = format!("{:#?}", &src);
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {
            str_source: src_format_error,
        })?;
        let binding: Vec<ObjectRange> = visit_items(&ast.items)?;
        let visited: &ObjectRange = binding
            .first()
            .ok_or(ErrorHandling::LineOutOfBounds { line_number: 0 })?;
        visit.push(ObjectRange {
            line_ranges: vec![LineRange::Start(range.start), LineRange::End(range.end)],
            names: vec![
                Name::TypeName(visited.object_type().unwrap()),
                Name::Name(visited.object_name().unwrap()),
            ],
        });
        Ok(visit)
    }

    fn rust_ast(src: &str) -> Result<File, ErrorHandling> {
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {
            str_source: &src.to_string(),
        })?;
        Ok(ast)
    }

    fn find_module_file(
        base_path: PathBuf,
        mod_name: String,
    ) -> Result<Option<PathBuf>, ErrorHandling> {
        let mut path = base_path;
        path.pop();
        let paths = [path.join(format!("{mod_name}.rs"))];
        for path in paths {
            if path.exists() {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
}

pub fn comment_lexer(source_vector: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
    let vectorized = FileExtractor::string_to_vector(source_vector);
    let mut comment_vector: Vec<ObjectRange> = Vec::new();
    let mut line_number = 0;
    for source in vectorized {
        line_number += 1;
        let tokenized = tokenize(&source);
        for each in tokenized {
            match each.kind {
                //Terminated indicates whether block comment ends in the same line it was initialized
                TokenKind::BlockComment { terminated } => {
                    if terminated {
                        comment_vector.push(ObjectRange {
                            line_ranges: vec![
                                LineRange::Start(line_number),
                                LineRange::End(line_number),
                            ],
                            names: vec![
                                Name::TypeName("CommentBlockSingeLine".to_string()),
                                Name::Name("Comment".to_string()),
                            ],
                        });
                    } else {
                        comment_vector.push(ObjectRange {
                            line_ranges: vec![LineRange::Start(line_number), LineRange::End(0)],
                            names: vec![
                                Name::TypeName("CommentBlockMultiLine".to_string()),
                                Name::Name("Comment".to_string()),
                            ],
                        });
                    };
                }
                TokenKind::Slash => {
                    comment_vector.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(line_number),
                            LineRange::End(line_number),
                        ],
                        names: vec![
                            Name::TypeName("CommentBlockMultiLineEnd".to_string()),
                            Name::Name("Refers to index - 1 (CommentBlockMultiLine)".to_string()),
                        ],
                    });
                }
                TokenKind::LineComment => {
                    comment_vector.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(line_number),
                            LineRange::End(line_number),
                        ],
                        names: vec![
                            Name::TypeName("LineComment".to_string()),
                            Name::Name("Comment".to_string()),
                        ],
                    });
                }

                TokenKind::Lifetime {
                    starts_with_number: _,
                } => {
                    comment_vector.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(line_number),
                            LineRange::End(line_number),
                        ],
                        names: vec![
                            Name::TypeName("LifetimeIndicator".to_string()),
                            Name::Name("Comment".to_string()),
                        ],
                    });
                }

                _ => {}
            }
        }
    }
    let target_type_name = "CommentBlockMultiLineEnd";
    let target_type_name2 = "CommentBlockMultiLine";
    let mut excess_index_pos = 0;
    if let Some(pos) = comment_vector.iter().position(|obj| {
        obj.names
            .iter()
            .any(|name| matches!(name, Name::TypeName(s) if s == target_type_name))
    }) {
        excess_index_pos = pos;
    } else {
        println!("No matching object found.");
    }
    if let Some(pos) = comment_vector.iter().position(|obj| {
        obj.names
            .iter()
            .any(|name| matches!(name, Name::TypeName(s) if s == target_type_name2))
    }) {
        comment_vector[pos]
            .line_ranges
            .retain(|r| !matches!(r, LineRange::End(0)));
        let borrow = &comment_vector[excess_index_pos].line_end().expect("err");
        comment_vector[pos]
            .line_ranges
            .push(LineRange::End(*borrow));
        comment_vector.remove(excess_index_pos);
    } else {
        println!("No matching object found.");
    }
    Ok(comment_vector)
}

fn function_parse(items: &[Item]) -> Result<FunctionSignature, ErrorHandling> {
    let mut vec_token_inputs: Vec<TokenStream> = Vec::new();
    if let Item::Fn(f) = &items[0] {
        //let input_tokens =  f.sig.inputs.clone().into_token_stream();
        let input_tokens = f.sig.inputs.iter();
        for each in input_tokens {
            match each {
                FnArg::Receiver(_) => {}
                FnArg::Typed(pat_type) => {
                    let input_tokens = pat_type.to_token_stream();
                    vec_token_inputs.push(input_tokens);
                }
            }
        }
        let output = &f.sig.output;
        if let ReturnType::Type(_, boxed_ty) = &output {
            let func = FunctionSignature {
                fn_input: fn_input(vec_token_inputs)?,
                fn_out: analyze_return_type(boxed_ty)?,
            };
                Ok(func)

        } else {
            Err(ErrorHandling::CouldNotGetLine)
        }

} else {
    Err(ErrorHandling::CouldNotGetLine)
}

}

fn fn_input(input_vector_stream: Vec<TokenStream>) -> Result<Vec<FnInputToken>, ErrorHandling> {
    let mut input_tokens: Vec<FnInputToken> = Vec::new();
    for input in input_vector_stream {
        let tokens: Vec<TokenTree> = input.into_iter().collect();
        for (i, token) in tokens.iter().enumerate() {
            if let TokenTree::Punct(punct) = token {
                if punct.as_char() == ':' && punct.spacing() != Spacing::Joint {
                    let before = tokens.get(i.wrapping_sub(1));
                    let after_tokens: Vec<TokenTree> = tokens.iter().skip(i + 1).cloned().collect();
                    let after_stream: TokenStream = after_tokens.into_iter().collect();
                    if let Some(before_token) = before {
                        let rm_space_from_before = remove_whitespace(before_token.to_string());
                        let rm_space_from_after = remove_whitespace(after_stream.to_string());
                        input_tokens.push({
                            FnInputToken {
                                input_name: rm_space_from_before?,
                                input_type: rm_space_from_after?,
                            }
                        });
                    }
                }
            }
        }
    }
    Ok(input_tokens)
}

fn remove_whitespace(s: String) -> Result<String, ErrorHandling> {
    Ok(s.chars().filter(|c| !c.is_whitespace()).collect())
}
fn analyze_return_type(ty: &Type) -> Result<FnOutputToken, ErrorHandling> {
    let mut kind = "Other".to_string();
    let mut output_type = ty.to_token_stream().to_string();
    let mut error_type = None;
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            let ident_str = segment.ident.to_string();
            match ident_str.as_str() {
                "Result" => {
                    kind = "Result".to_string();

                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args,
                        ..
                    }) = &segment.arguments
                    {
                        let mut args = args.iter();
                        if let Some(ok_ty) = args.next() {
                            output_type = remove_whitespace(ok_ty.to_token_stream().to_string())?;
                        }
                        if let Some(err_ty) = args.next() {
                            error_type = Some(err_ty.to_token_stream().to_string());
                        }
                    }
                }
                "Option" => {
                    kind = "Option".to_string();

                    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args,
                        ..
                    }) = &segment.arguments
                    {
                        if let Some(inner_ty) = args.first() {
                            output_type =
                                remove_whitespace(inner_ty.to_token_stream().to_string())?;
                        }
                    }
                }
                _ => {
                    kind = "Other".to_string();
                    output_type = ty.to_token_stream().to_string();
                }
            }
        }
    }
    Ok(FnOutputToken {
        kind,
        output_type,
        error_type,
    })
}

fn visit_items(items: &[Item]) -> Result<Vec<ObjectRange>, ErrorHandling> {
    let mut object_line: Vec<ObjectRange> = Vec::new();

    for item in items {
        match item {
            Item::Struct(s) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(s.span().start().line),
                        LineRange::End(s.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("struct".to_string()),
                        Name::Name(s.ident.to_string()),
                    ],
                });
            }
            Item::Enum(e) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(e.span().start().line),
                        LineRange::End(e.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("enum".to_string()),
                        Name::Name(e.ident.to_string()),
                    ],
                });
            }
            Item::Fn(f) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(f.span().start().line),
                        LineRange::End(f.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("fn".to_string()),
                        Name::Name(f.sig.ident.to_string()),
                    ],
                });
            }
            Item::Mod(m) => match &m.content {
                Some((_, items)) => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.span().start().line),
                            LineRange::End(m.span().end().line),
                        ],
                        names: vec![
                            Name::TypeName("mod".to_string()),
                            Name::Name(m.ident.to_string()),
                        ],
                    });
                    object_line.extend(visit_items(items)?);
                }
                None => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.span().start().line),
                            LineRange::End(m.span().end().line),
                        ],
                        names: vec![
                            Name::TypeName("mod".to_string()),
                            Name::Name(m.ident.to_string()),
                        ],
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
                        names: vec![
                            Name::TypeName("use".to_string()),
                            Name::Name(path.ident.to_string()),
                        ],
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
                    names: vec![Name::TypeName("impl".to_string()), Name::Name(trait_name)],
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
                                    Name::TypeName("fn".to_string()),
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
                                    Name::TypeName("const".to_string()),
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
                                    Name::TypeName("type".to_string()),
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
                                    Name::TypeName("macro".to_string()),
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
                                names: vec![
                                    Name::TypeName("verbatim".to_string()),
                                    Name::Name(v.to_string()),
                                ],
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
                    names: vec![
                        Name::TypeName("trait".to_string()),
                        Name::Name(t.ident.to_string()),
                    ],
                });
            }
            Item::Type(t) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(t.span().start().line),
                        LineRange::End(t.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("type".to_string()),
                        Name::Name(t.ident.to_string()),
                    ],
                });
            }
            Item::Union(u) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(u.span().start().line),
                        LineRange::End(u.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("union".to_string()),
                        Name::Name(u.ident.to_string()),
                    ],
                });
            }
            Item::Const(c) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(c.span().start().line),
                        LineRange::End(c.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("const".to_string()),
                        Name::Name(c.ident.to_string()),
                    ],
                });
            }
            Item::Macro(m) => {
                object_line.push(ObjectRange {
                    line_ranges: vec![
                        LineRange::Start(m.span().start().line),
                        LineRange::End(m.span().end().line),
                    ],
                    names: vec![
                        Name::TypeName("macro".to_string()),
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
                        Name::TypeName("extern crate".to_string()),
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
                    names: vec![
                        Name::TypeName("static".to_string()),
                        Name::Name(s.ident.to_string()),
                    ],
                });
            }
            _ => println!("Other item"),
        }
    }

    Ok(object_line)
}
