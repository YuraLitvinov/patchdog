use crate::error::{ErrorHandling, InvalidIoOperationsSnafu};
use crate::object_range::{Name, ObjectRange};
use proc_macro2::TokenStream;
use proc_macro2::{Spacing, TokenTree};
use quote::ToTokens;
use serde::Serialize;
use std::ops::Range;
use rustc_lexer::{tokenize, TokenKind};
use std::path::{Path, PathBuf};
use crate::file_parsing::{FileExtractor, Files};
use std::fs;
use syn::spanned::Spanned;
use syn::{AngleBracketedGenericArguments, PathArguments, Type, TypePath};
use syn::{File, ReturnType};
use syn::{FnArg, parse_str};
use syn::{ImplItem, Item};
use tracing::{Level, event};
use snafu::ResultExt;
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

///   Parses a Rust file from the given `src` path and extracts all identifiable Rust items within it.
///   It reads the file content into a string, then uses `syn` to parse this string into a Rust Abstract Syntax Tree (AST).
///   The function then traverses the AST to identify structures like functions, structs, enums, etc., and returns a vector of `ObjectRange` structs, each detailing an item's type, name, and line range.
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let file = fs::read_to_string(src)
            .context(InvalidIoOperationsSnafu { path: src })?;
        let ast: File = parse_str(&file)?;
        visit_items(&ast.items)
    }

/// Parses all Rust items, including code structures (functions, structs, enums, etc.) and comments, from a given source string.
/// It first parses the source into an AST, then extracts items from the AST.
/// Concurrently, it lexes comments from the raw source string.
/// Finally, it combines the extracted code items and comments into a single vector of `ObjectRange` and sorts them by their starting line number.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) containing the Rust source code.
///
/// # Returns
///
/// A `Result<Vec<ObjectRange>, ErrorHandling>`:
/// - `Ok(Vec<ObjectRange>)`: A sorted vector of `ObjectRange` structs, representing all parsed code items and comments.
/// - `Err(ErrorHandling)`: If parsing the AST or lexing comments fails.
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let ast: File = parse_str(src)?;
        let mut comments = comment_lexer(src)?;
        let mut visited = visit_items(&ast.items)?;
        visited.append(&mut comments);
        visited.sort_by(|a, b| a.line_ranges.start.cmp(&b.line_ranges.start));

        Ok(visited)
    }

/// Parses a Rust function's signature from a given source string.
/// It converts the source string into a Rust AST (`syn::File`) and then specifically extracts the function signature details.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) containing the Rust function code.
///
/// # Returns
///
/// A `Result<FunctionSignature, ErrorHandling>`:
/// - `Ok(FunctionSignature)`: A struct containing the parsed input parameters and return type information for the function.
/// - `Err(ErrorHandling)`: If parsing the AST fails or the provided `src` does not represent a valid function.
    fn rust_function_parser(src: &str) -> Result<FunctionSignature, ErrorHandling> {
        let ast: File = parse_str(src)?;
        function_parse(&ast.items)
    }

/// Parses a single Rust item (e.g., function, struct, enum) from a given source string.
/// It converts the source string into a Rust AST (`syn::File`) and extracts the first item it finds.
/// The function then creates an `ObjectRange` representing the line range and name of this item.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) containing the Rust item's code.
///
/// # Returns
///
/// A `Result<ObjectRange, ErrorHandling>`:
/// - `Ok(ObjectRange)`: A struct containing the line range (start and end lines) and the name (type and identifier) of the first parsed Rust item.
/// - `Err(ErrorHandling::LineOutOfBounds)`: If no items are found in the parsed source.
/// - `Err(ErrorHandling)`: If parsing the AST fails.
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling> {
        let ast: File = parse_str(src)?;
        let binding: Vec<ObjectRange> = visit_items(&ast.items)?;
        let visited: &ObjectRange = binding
            .first()
            .ok_or(ErrorHandling::LineOutOfBounds { line_number: 0 })?;
        Ok(ObjectRange {
            line_ranges: Range {
                start: visited.line_start(),
                end: visited.line_end(),
            },
            names: Name {
                type_name: visited.object_type(),
                name: visited.object_name(),
            },
        })
    }

/// Parses a Rust source string into its Abstract Syntax Tree (AST) representation.
/// This function is a wrapper around `syn::parse_str` to simplify AST parsing.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) containing the Rust source code.
///
/// # Returns
///
/// A `Result<syn::File, ErrorHandling>`:
/// - `Ok(syn::File)`: The parsed AST of the Rust code.
/// - `Err(ErrorHandling)`: If parsing fails (e.g., due to invalid Rust syntax).
    fn rust_ast(src: &str) -> Result<File, ErrorHandling> {
        let ast: File = parse_str(src)?;
        Ok(ast)
    }

/// Attempts to find the file path for a Rust module given a base path and the module name.
/// It assumes the module file will be named `{mod_name}.rs` and located in the same directory as the `base_path`'s parent.
///
/// # Arguments
///
/// * `base_path` - A `PathBuf` representing the path of the file where the module is declared (e.g., `lib.rs`).
/// * `mod_name` - A `String` representing the name of the module to find (e.g., "data" for `mod data;`).
///
/// # Returns
///
/// A `Result<Option<PathBuf>, ErrorHandling>`:
/// - `Ok(Some(PathBuf))`: If the module file is found.
/// - `Ok(None)`: If the module file does not exist at the expected location.
/// - `Err(ErrorHandling)`: If an I/O error occurs while checking file existence.
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

/// Lexes a Rust source string to identify and extract information about comments.
/// It tokenizes each line of the source and categorizes comments into single-line block, multi-line block (start and end),
/// and single-line comments. It also identifies 'LifetimeIndicator' tokens.
/// Special handling is included to correctly associate the start and end lines of multi-line block comments.
///
/// # Arguments
///
/// * `source_vector` - A string slice (`&str`) containing the Rust source code.
///
/// # Returns
///
/// A `Result<Vec<ObjectRange>, ErrorHandling>`:
/// - `Ok(Vec<ObjectRange>)`: A vector of `ObjectRange` structs, where each represents a detected comment or lifetime indicator with its line range and type.
/// - `Err(ErrorHandling)`: If string processing fails (e.g., `FileExtractor::string_to_vector` or `tokenize`).
pub fn comment_lexer(source_vector: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
    let vectorized = FileExtractor::string_to_vector(source_vector);
    let mut comment_vector: Vec<ObjectRange> = Vec::new();
    for (line_number, source) in vectorized.into_iter().enumerate() {
        let tokenized = tokenize(&source);
        for each in tokenized {
            match each.kind {
                //Terminated indicates whether block comment ends in the same line it was initialized
                TokenKind::BlockComment { terminated } => {
                    if terminated {
                        comment_vector.push(ObjectRange {
                            line_ranges: Range { start: line_number, end: line_number },
                            names: Name {
                                type_name: "CommentBlockSingeLine".to_string(),
                                name: "Comment".to_string(),
                            },
                        });
                    } else {
                        comment_vector.push(ObjectRange {
                            line_ranges: Range { start: line_number, end: 0 },
                            names: Name {
                                type_name: "CommentBlockMultiLine".to_string(),
                                name: "Comment".to_string(),
                            },
                        });
                    };
                }
                TokenKind::Slash => {
                    comment_vector.push(ObjectRange {
                        line_ranges: Range { start: line_number, end: line_number },
                        names: Name {
                            type_name: "CommentBlockMultiLineEnd".to_string(),
                            name: "Refers to index - 1 (CommentBlockMultiLine)".to_string(),
                        },
                    });
                }
                TokenKind::LineComment => {
                    comment_vector.push(ObjectRange {
                        line_ranges: Range { start: line_number, end: line_number },
                        names: Name {
                            type_name: "LineComment".to_string(),
                            name: "Comment".to_string(),
                        },
                    });
                }

                TokenKind::Lifetime {
                    starts_with_number: _,
                } => {
                    comment_vector.push(ObjectRange {
                        line_ranges: Range { start: line_number, end: line_number },
                        names: Name {
                            type_name: "LifetimeIndicator".to_string(),
                            name: "Comment".to_string(),
                        },
                    });
                }

                _ => {}
            }
        }
    }
    let multi_line = "CommentBlockMultiLine";
    let multi_line_end = "CommentBlockMultiLineEnd";
    let mut found_position = 0;
    if let Some(pos) = comment_vector.iter().position(|obj| obj.names.type_name == multi_line) {
        found_position = pos;
    }
    if let Some(pos) = comment_vector.iter().position(|obj| obj.names.type_name == multi_line_end) {
        comment_vector[found_position].line_ranges.end = comment_vector[pos].line_end();
        comment_vector.remove(pos);
    }

    Ok(comment_vector)
}

/// Parses a slice of `syn::Item`s to extract the signature (inputs and output) of the first function found.
/// It iterates through function arguments to collect input `TokenStream`s and then analyzes the return type.
/// Supports `Result`, `Option`, and default return types.
///
/// # Arguments
///
/// * `items` - A slice of `syn::Item`s, expected to contain at least one `Item::Fn`.
///
/// # Returns
///
/// A `Result<FunctionSignature, ErrorHandling>`:
/// - `Ok(FunctionSignature)`: A struct containing details about the function's input parameters and return type.
/// - `Err(ErrorHandling::NotFunction)`: If the first item is not a function.
/// - `Err(ErrorHandling::CouldNotGetObject)`: If the return type cannot be analyzed.
/// - `Err(ErrorHandling)`: If `fn_input` or `analyze_return_type` fails.
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

fn fn_input(input_vector_stream: Vec<TokenStream>) -> Result<Vec<FnInputToken>, ErrorHandling> {
    let mut input_tokens: Vec<FnInputToken> = Vec::new();
    for input in input_vector_stream {
        let tokens: Vec<TokenTree> = input.into_iter().collect();
        for (i, token) in tokens.iter().enumerate() {
            if let TokenTree::Punct(punct) = token
                && punct.as_char() == ':' && punct.spacing() != Spacing::Joint {
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

    Ok(input_tokens)
}

/// Removes all whitespace characters from a given string.
/// This function is typically used for canonicalizing strings by stripping unnecessary spaces, tabs, and newlines.
///
/// # Arguments
///
/// * `s` - The `String` from which whitespace should be removed.
///
/// # Returns
///
/// A `Result<String, ErrorHandling>`:
/// - `Ok(String)`: A new `String` with all whitespace characters filtered out.
/// - `Err(ErrorHandling)`: This function is currently infallible but returns a `Result` for consistency.
pub fn remove_whitespace(s: String) -> Result<String, ErrorHandling> {
    Ok(s.chars().filter(|c| !c.is_whitespace()).collect())
}

fn analyze_return_type(ty: &Type) -> Result<FnOutputToken, ErrorHandling> {
    let mut kind = "Other".to_string();
    let mut output_type = ty.to_token_stream().to_string();
    let mut error_type = None;
    if let Type::Path(TypePath { path, .. }) = ty &&
        let Some(segment) = path.segments.last() {
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
                        && let Some(inner_ty) = args.first() {
                            output_type =
                                remove_whitespace(inner_ty.to_token_stream().to_string())?;
                        }
                }
                _ => {
                    kind = "Other".to_string();
                    output_type = ty.to_token_stream().to_string();
                }
            }
        
    }
    Ok(FnOutputToken {
        kind,
        output_type,
        error_type,
    })
}

/// Visits a slice of `syn::Item`s to extract information about Rust code objects.
/// It iterates through various types of items (structs, enums, functions, modules, uses, impls, traits, types, unions, consts, macros, extern crates, statics).
/// For each recognized item, it creates an `ObjectRange` containing its line range (start and end lines) and its name (type and identifier).
/// Special handling is included for `impl` blocks to also parse items within the implementation.
/// Recursive calls are made for module items with content.
///
/// # Arguments
///
/// * `items` - A slice of `syn::Item`s representing the parsed items from a Rust file or module.
///
/// # Returns
///
/// A `Result<Vec<ObjectRange>, ErrorHandling>`:
/// - `Ok(Vec<ObjectRange>)`: A vector of `ObjectRange` structs, each representing a parsed Rust code object with its line range and name.
/// - `Err(ErrorHandling)`: If nested parsing (e.g., within modules) fails or other internal errors occur.
fn visit_items(items: &[Item]) -> Result<Vec<ObjectRange>, ErrorHandling> {
    let mut object_line: Vec<ObjectRange> = Vec::new();

    for item in items {
        match item {
            Item::Struct(s) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: s.span().start().line, end: s.span().end().line },
                    names: Name {
                        type_name: "struct".to_string(),
                        name: s.ident.to_string(),
                    },
                });
            }
            Item::Enum(e) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: e.span().start().line, end: e.span().end().line },
                    names: Name {
                        type_name: "enum".to_string(),
                        name: e.ident.to_string(),
                    },
                });
            }
            Item::Fn(f) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: f.span().start().line, end: f.span().end().line },
                    names: Name {
                        type_name: "fn".to_string(),
                        name: f.sig.ident.to_string(),
                    },
                });
            }
            Item::Mod(m) => match &m.content {
                Some((_, items)) => {
                    object_line.push(ObjectRange {
                        line_ranges: Range { start: m.span().start().line, end: m.span().end().line },
                        names: Name {
                            type_name: "mod".to_string(),
                            name: m.ident.to_string(),
                        },
                    });
                    object_line.extend(visit_items(items)?);
                }
                None => {
                    object_line.push(ObjectRange {
                        line_ranges: Range { start: m.span().start().line, end: m.span().end().line },
                        names: Name {
                            type_name: "mod".to_string(),
                            name: m.ident.to_string(),
                        },
                    });
                }
            },
            Item::Use(u) => {
                if let syn::UseTree::Path(path) = u.tree.to_owned() {
                    object_line.push(ObjectRange {
                        line_ranges: Range { start: path.span().start().line, end: path.span().end().line },
                        names: Name {
                            type_name: "use".to_string(),
                            name: path.ident.to_string(),
                        },
                    });
                }
            }
            Item::Impl(i) => {
                let trait_name = if let Some((_, path, _)) = &i.trait_ {
                    if let Some(seg) = path.segments.last() {
                        seg.ident.to_string()
                    } else {
                        "matches struct".to_string()
                    }
                } else {
                    "matches struct".to_string()
                };
                object_line.push(ObjectRange {
                    line_ranges: Range { start: i.span().start().line, end: i.span().end().line },
                    names: Name {
                        type_name: "impl".to_string(),
                        name: trait_name,
                    },
                });
                for each_block in &i.items {
                    match each_block {
                        ImplItem::Fn(f) => {
                            object_line.push(ObjectRange {
                                line_ranges: Range { start: f.span().start().line, end: f.span().end().line },
                                names: Name {
                                    type_name: "fn".to_string(),
                                    name: f.sig.ident.to_string(),
                                },
                            });
                        }
                        ImplItem::Const(c) => {
                            object_line.push(ObjectRange {
                                line_ranges: Range { start: c.span().start().line, end: c.span().end().line },
                                names: Name {
                                    type_name: "const".to_string(),
                                    name: c.ident.to_string(),
                                },
                            });
                        }
                        ImplItem::Type(t) => {
                            object_line.push(ObjectRange {
                                line_ranges: Range { start: t.span().start().line, end: t.span().end().line },
                                names: Name {
                                    type_name: "type".to_string(),
                                    name: t.ident.to_string(),
                                },
                            });
                        }
                        ImplItem::Macro(m) => {
                            object_line.push(ObjectRange {
                                line_ranges: Range { start: m.span().start().line, end: m.span().end().line },
                                names: Name {
                                    type_name: "macro".to_string(),
                                    name: format!("{:?}", m.mac.path),
                                },
                            });
                        }
                        ImplItem::Verbatim(v) => {
                            object_line.push(ObjectRange {
                                line_ranges: Range { start: v.span().start().line, end: v.span().end().line },
                                names: Name {
                                    type_name: "verbatim".to_string(),
                                    name: v.to_string(),
                                },
                            });
                        }
                        _ => event!(Level::INFO, "Other impl object"),
                    }
                }
            }
            Item::Trait(t) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: t.span().start().line, end: t.span().end().line },
                    names: Name {
                        type_name: "trait".to_string(),
                        name: t.ident.to_string(),
                    },
                });
            }
            Item::Type(t) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: t.span().start().line, end: t.span().end().line },
                    names: Name {
                        type_name: "type".to_string(),
                        name: t.ident.to_string(),
                    },
                });
            }
            Item::Union(u) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: u.span().start().line, end: u.span().end().line },
                    names: Name {
                        type_name: "union".to_string(),
                        name: u.ident.to_string(),
                    },
                });
            }
            Item::Const(c) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: c.span().start().line, end: c.span().end().line },
                    names: Name {
                        type_name: "const".to_string(),
                        name: c.ident.to_string(),
                    },
                });
            }
            Item::Macro(m) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: m.span().start().line, end: m.span().end().line },
                    names: Name {
                        type_name: "macro".to_string(),
                        name: format!("{:?}", m.mac.path),
                    },
                });
            }
            Item::ExternCrate(c) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: c.span().start().line, end: c.span().end().line },
                    names: Name {
                        type_name: "extern crate".to_string(),
                        name: c.ident.to_string(),
                    },
                });
            }
            Item::Static(s) => {
                object_line.push(ObjectRange {
                    line_ranges: Range { start: s.span().start().line, end: s.span().end().line },
                    names: Name {
                        type_name: "static".to_string(),
                        name: s.ident.to_string(),
                    },
                });
            }
            _ => event!(Level::INFO, "Other item"),
        }
    }

    Ok(object_line)
}
