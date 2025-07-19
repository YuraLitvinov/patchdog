use crate::error::{ErrorHandling, InvalidIoOperationsSnafu, InvalidItemParsingSnafu};
use crate::object_range::{LineRange, Name, ObjectRange};
use proc_macro2::TokenStream;
use proc_macro2::{Spacing, TokenTree};
use quote::ToTokens;
use snafu::ResultExt;
use std::path::PathBuf;
use std::{vec, fs};
use syn::{parse_str, FnArg};
use syn::spanned::Spanned;
use syn::{AngleBracketedGenericArguments, PathArguments, Type, TypePath};
use syn::{File, ReturnType};
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
    fn rust_function_parser(src: &str) -> Result<Vec<FunctionSignature>, ErrorHandling>;
    fn parse_rust_file(src: &PathBuf) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn rust_ast(src: &str) -> Result<File, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {
    fn parse_rust_file(src: &PathBuf) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let file = fs::read_to_string(&src).context(InvalidIoOperationsSnafu)?;
        let ast: File = parse_str(&file).context(InvalidItemParsingSnafu {str_source: src})?;
        Ok(visit_items(&ast.items)?)   
    }

    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let src_format_error = format!("{:#?}", &src);
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {str_source: src_format_error})?;
        Ok(visit_items(&ast.items)?)
    }
    fn rust_function_parser(src: &str) -> Result<Vec<FunctionSignature>, ErrorHandling> {
        let src_format_error = format!("{}", &src);
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {str_source: src_format_error})?;
        Ok(function_parse(&ast.items)?)
    }
        fn rust_ast(src: &str) -> Result<File, ErrorHandling> {
        let src_format_error = format!("{}", &src);
        let ast: File = parse_str(src).context(InvalidItemParsingSnafu {str_source: src_format_error})?;
        Ok(ast)
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
fn function_parse(items: &[Item]) -> Result<Vec<FunctionSignature>,ErrorHandling> {
    let mut fn_sig: Vec<FunctionSignature> = Vec::new();
    let mut vec_token_inputs: Vec<TokenStream> = Vec::new();
    if let Item::Fn(f) = &items[0] {
        //let input_tokens =  f.sig.inputs.clone().into_token_stream();
        let input_tokens =  f.sig.inputs.iter();
        for each in input_tokens {
        match each {
            FnArg::Receiver(_) => {},
            FnArg::Typed(pat_type) => {
               let input_tokens = pat_type.to_token_stream();
               vec_token_inputs.push(input_tokens);
            }

        }
    }
        let output = &f.sig.output;
        if let ReturnType::Type(_, boxed_ty) = &output {
        fn_sig.push(FunctionSignature {
        fn_input: fn_input(vec_token_inputs)?,
        fn_out: analyze_return_type(boxed_ty)?,
        });
    }
}
Ok(fn_sig)
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
                            output_type = remove_whitespace(inner_ty.to_token_stream().to_string())?;
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
                            LineRange::End(m.span().end().line),
                        ],
                        names: vec![Name::TypeName("mod"), Name::Name(m.ident.to_string())],
                    });
                    object_line.extend(visit_items(items)?);
                }
                None => {
                    object_line.push(ObjectRange {
                        line_ranges: vec![
                            LineRange::Start(m.span().start().line),
                            LineRange::End(m.span().end().line),
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

    Ok(object_line)
}
