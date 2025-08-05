use crate::error::ErrorHandling;
use crate::file_parsing::{FileExtractor, Files};
use crate::object_range::{LineRange, Name, ObjectRange};
use proc_macro2::TokenStream;
use proc_macro2::{Spacing, TokenTree};
use quote::ToTokens;
use rustc_lexer::TokenKind;
use rustc_lexer::tokenize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{fs, vec};
use syn::spanned::Spanned;
use syn::{AngleBracketedGenericArguments, PathArguments, Type, TypePath};
use syn::{File, ReturnType};
use syn::{FnArg, parse_str};
use syn::{ImplItem, Item};
use tracing::{Level, event};
/*
1. Парсер патчей
2. Раст парсер
3. Предподготовка запросов к ЛЛМ
    1. Хеширование запросов
    2. Сериализация запросов
4. Обработка ответа
    1. Сопоставление данных полученных и переданных
5. Запись ответа
*/
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, Serialize, PartialEq)]
pub struct FunctionSignature {
    fn_input: Vec<FnInputToken>,
    fn_out: FnOutputToken,
}
#[derive(Debug, Clone)]
#[allow(dead_code)]
#[derive(serde::Deserialize, Serialize, PartialEq)]
struct FnInputToken {
    input_name: String,
    input_type: String,
}
#[derive(Debug, Clone)]
#[allow(dead_code)]
#[derive(serde::Deserialize, Serialize, PartialEq)]
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
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {
    /// Parses a Rust file from a given path, converting its content into an Abstract Syntax Tree (AST).
    /// It then visits the items in the AST to extract their line ranges and names, returning them as a vector of `ObjectRange` structs.
    ///
    /// # Arguments
    ///
    /// * `src`: A reference to a `Path` pointing to the Rust source file.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<ObjectRange>` representing the parsed items, or an `ErrorHandling` if the file cannot be read or parsing fails.
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let file = fs::read_to_string(src)?;
        let ast: File = parse_str(&file)?;
        visit_items(&ast.items)
    }

    /// Parses a Rust source code string to extract all top-level Rust items and comments.
    /// It converts the source into an AST, extracts items, then lexes comments separately, combines both sets of `ObjectRange` structs, and sorts them by their starting line number.
    ///
    /// # Arguments
    ///
    /// * `src`: A string slice (`&str`) containing the Rust source code.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<ObjectRange>` of all identified code items and comments, sorted by line number, or an `ErrorHandling` if parsing or comment lexing fails.
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let ast: File = parse_str(src)?;
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

    /// Parses a Rust source code string and extracts the signature details of the first function found.
    /// It converts the source into an AST and then delegates to `function_parse` to extract input arguments and return type information.
    ///
    /// # Arguments
    ///
    /// * `src`: A string slice (`&str`) containing the Rust source code to parse.
    ///
    /// # Returns
    ///
    fn rust_function_parser(src: &str) -> Result<FunctionSignature, ErrorHandling> {
        let ast: File = parse_str(src)?;
        function_parse(&ast.items)
    }

    /// Parses a Rust source code string to identify and extract information about its first top-level item.
    /// It converts the source into an AST, extracts items, and then returns an `ObjectRange` containing the line ranges, type, and name of the very first item found.
    ///
    /// # Arguments
    ///
    /// * `src`: A string slice (`&str`) containing the Rust source code.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `ObjectRange` struct for the first item, or an `ErrorHandling` if the source is empty or parsing fails.
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling> {
        let ast: File = parse_str(src)?;
        let binding: Vec<ObjectRange> = visit_items(&ast.items)?;
        let visited: &ObjectRange = binding
            .first()
            .ok_or(ErrorHandling::LineOutOfBounds { line_number: 0 })?;
        Ok(ObjectRange {
            line_ranges: vec![
                LineRange::Start(visited.line_start().unwrap()),
                LineRange::End(visited.line_end().unwrap()),
            ],
            names: vec![
                Name::TypeName(visited.object_type().unwrap()),
                Name::Name(visited.object_name().unwrap()),
            ],
        })
    }

    /// Parses a Rust source code string into a `syn::File` Abstract Syntax Tree (AST).
    /// This function serves as a wrapper around `syn::parse_str` for converting code strings into a structured representation.
    ///
    /// # Arguments
    ///
    /// * `src`: A string slice (`&str`) containing the Rust source code.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `syn::File` AST on success, or an `ErrorHandling` if parsing fails (e.g., due to syntax errors).
    fn rust_ast(src: &str) -> Result<File, ErrorHandling> {
        let ast: File = parse_str(src)?;
        Ok(ast)
    }

    /// Searches for a Rust module file (`.rs`) within the directory containing the given `base_path`.
    /// It constructs potential file paths based on the `mod_name` (e.g., `mod_name.rs`) and checks if they exist.
    ///
    /// # Arguments
    ///
    /// * `base_path`: A `PathBuf` representing the path of the file where the module is declared (used to determine the search directory).
    /// * `mod_name`: A `String` containing the name of the module to search for.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `Option<PathBuf>`: `Some(path)` if the module file is found, `None` if it's not found, or an `ErrorHandling` if an error occurs during path manipulation.
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

/// Lexes a Rust source code string to identify and categorize different types of comments and lifetime indicators.
/// It tokenizes each line and creates `ObjectRange` structs for line comments, block comments (single or multi-line), and lifetime indicators, including logic to correctly identify the end line of multi-line block comments.
///
/// # Arguments
///
/// * `source_vector`: A string slice (`&str`) containing the Rust source code.
///
/// # Returns
///
/// A `Result` containing a `Vec<ObjectRange>` representing the identified comments and lifetimes, or an `ErrorHandling` if an error occurs during tokenization or line range processing.
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
    }

    Ok(comment_vector)
}

/// Parses a slice of `syn::Item` structs to extract function signature information.
/// It specifically expects the first item in the slice to be a function (`Item::Fn`) and extracts its input arguments and return type details into a `FunctionSignature` struct.
///
/// # Arguments
///
/// * `items`: A slice of `syn::Item` structs, typically representing items from a parsed Rust file or module.
///
/// # Returns
///
/// A `Result` containing a `FunctionSignature` struct on success, or an `ErrorHandling` if the first item is not a function or if parsing its signature fails.
fn function_parse(items: &[Item]) -> Result<FunctionSignature, ErrorHandling> {
    let mut vec_token_inputs: Vec<TokenStream> = Vec::new();
    let default_return = FnOutputToken {
        kind: "Default".to_string(),
        output_type: "()".to_string(),
        error_type: Some("None".to_string()),
    };
    if let Item::Fn(f) = &items[0] {
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
            if let ReturnType::Default = &output {
                let func = FunctionSignature {
                    fn_input: fn_input(vec_token_inputs)?,
                    fn_out: default_return,
                };
                return Ok(func);
            }
            Err(ErrorHandling::CouldNotGetObject {
                err_kind: format!("{:?} Name: {}", output, f.sig.ident),
            })
        }
    } else {
        event!(Level::ERROR, "{items:#?}");
        Err(ErrorHandling::NotFunction)
    }
}

/// Parses a vector of `proc_macro2::TokenStream`s, typically representing function input arguments, to extract their names and types.
/// It iterates through the tokens, identifies the ':' separator, and extracts the token before it as the input name and the tokens after it as the input type, removing all whitespace.
///
/// # Arguments
///
/// * `input_vector_stream`: A `Vec<TokenStream>` where each `TokenStream` represents a single function input argument.
///
/// # Returns
///
/// A `Result` containing a `Vec<FnInputToken>` with the extracted input names and types, or an `ErrorHandling` if token parsing or whitespace removal fails.
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

/// Removes all whitespace characters from a given `String`.
/// This includes spaces, tabs, newlines, and other Unicode whitespace.
///
/// # Arguments
///
/// * `s`: The `String` from which whitespace should be removed.
///
/// # Returns
///
/// A `Result` containing the new `String` with all whitespace removed, or an `ErrorHandling` if an unexpected error occurs during string processing (though unlikely for this operation).
pub fn remove_whitespace(s: String) -> Result<String, ErrorHandling> {
    Ok(s.chars().filter(|c| !c.is_whitespace()).collect())
}
/// Analyzes a `syn::Type` to determine its structural kind (e.g., `Result`, `Option`, or `Other`), its primary output type, and an optional error type if it's a `Result` type.
/// It extracts the type arguments for `Result` and `Option` to provide more granular information.
///
/// # Arguments
///
/// * `ty`: A reference to the `syn::Type` to be analyzed.
///
/// # Returns
///
/// A `Result` containing a `FnOutputToken` struct with the analyzed type information, or an `ErrorHandling` if an error occurs during token stream conversion or whitespace removal.
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

/// Recursively visits items within a Rust syntax tree (`syn::Item`s) to extract their type, name, and line range information.
/// It categorizes various Rust constructs like structs, enums, functions, modules, `impl` blocks (and their contained items), `use` statements, traits, types, unions, constants, macros, `extern crate` declarations, and statics.
/// For modules and `impl` blocks, it recursively processes their inner items.
///
/// # Arguments
///
/// * `items`: A slice of `syn::Item` structs to be visited.
///
/// # Returns
///
/// A `Result` containing a `Vec<ObjectRange>` where each `ObjectRange` represents a discovered code item with its line bounds and identifying names, or an `ErrorHandling` if any parsing or line range extraction fails.
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
                        _ => event!(Level::INFO, "Other impl object"),
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
            _ => event!(Level::INFO, "Other item"),
        }
    }

    Ok(object_line)
}
