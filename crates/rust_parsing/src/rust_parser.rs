use crate::error::{ErrorHandling, InvalidIoOperationsSnafu};
use crate::file_parsing::{FileExtractor, Files};
use crate::object_range::{Name, ObjectRange};
use ra_ap_ide::TextRange;
use ra_ap_syntax::ast::{HasModuleItem, HasName};
use ra_ap_syntax::{AstNode, ToSmolStr};
use rayon::prelude::*;
use rustc_lexer::{TokenKind, tokenize};
use serde::Serialize;
use snafu::ResultExt;
use std::collections::HashMap;
use std::fs;
use std::ops::Range;
use std::path::{Path};
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
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct AnalyzerRange {
    pub range: TextRange,
    pub names: Name,
}
pub trait RustParser {
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling>;
    fn textrange_into_linerange(range: TextRange, src: &str) -> Range<usize>;
    fn parse_result_items(src: &str) -> Result<HashMap<TextRange, AnalyzerRange>, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {

/// Parses a Rust source file from a given path and extracts a vector of `ObjectRange` items, each representing a distinct code object (e.g., function, struct) and its line range. This function first reads the file content, then uses a Rust item parser to identify and extract the structural elements.
/// It then maps the byte-based text ranges provided by the parser to line-based ranges, making them more human-readable and suitable for operations involving line numbers. This is a core function for understanding the structure of a Rust file.
///
/// # Arguments
///
/// * `src` - A reference to a `Path` pointing to the Rust source file.
///
/// # Returns
///
/// A `Result<Vec<ObjectRange>, ErrorHandling>` containing a vector of `ObjectRange` objects representing the parsed code items, or an `ErrorHandling` if file reading or parsing fails.
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let file = fs::read_to_string(src).context(InvalidIoOperationsSnafu { path: src })?;
        let visited = Self::parse_result_items(&file)?
            .par_iter()
            .map(|val| {
                let line_ranges = Self::textrange_into_linerange(*val.0, &file);
                ObjectRange {
                    line_ranges,
                    names: val.1.names.clone(),
                }
            })
            .collect::<Vec<ObjectRange>>();
        Ok(visited)
    }

/// Parses a Rust source string to identify all significant Rust items and comments within it.
/// It first extracts comments using `comment_lexer` and then parses other structural items (functions, structs, etc.) using `parse_result_items`.
/// The function then merges these two sets of identified ranges, converts byte-based ranges to line-based ranges, and sorts the final list by their starting line numbers, providing a comprehensive overview of the code structure.
///
/// # Arguments
///
/// * `src` - A string slice representing the Rust source code.
///
/// # Returns
///
/// A `Result` which is `Ok(Vec<ObjectRange>)` on success, containing a sorted vector of `ObjectRange` structs for all identified items and comments, or an `ErrorHandling` enum if an error occurs during parsing.
    fn parse_all_rust_items(src: &str) -> Result<Vec<ObjectRange>, ErrorHandling> {
        let mut comments = comment_lexer(src)?;
        let mut visited = Self::parse_result_items(src)?
            .par_iter()
            .map(|val| {
                let line_ranges = Self::textrange_into_linerange(*val.0, src);
                ObjectRange {
                    line_ranges,
                    names: val.1.names.clone(),
                }
            })
            .collect::<Vec<ObjectRange>>();
        visited.append(&mut comments);
        visited.sort_by(|a, b| a.line_ranges.start.cmp(&b.line_ranges.start));

        Ok(visited)
    }
/// Converts a byte-offset based `TextRange` into a human-readable, 1-based line number `Range<usize>`.
/// This utility function computes the start and end line numbers by first determining the line start offsets in the source string.
/// It then translates the byte offsets of the `TextRange` into their corresponding line numbers, ensuring the output is always 1-based for user-friendliness.
///
/// # Arguments
///
/// * `range` - The `TextRange` to convert, specifying a section of text by byte offsets.
/// * `src` - The complete source code string, used to compute line starts.
///
/// # Returns
///
/// A `Range<usize>` where `start` and `end` are 1-based line numbers.
    fn textrange_into_linerange(range: TextRange, src: &str) -> Range<usize> {
        let line_starts = compute_line_starts(src);

        let start = offset_to_line(range.start().into(), &line_starts);
        let end = offset_to_line(range.end().into(), &line_starts);

        // Always return consistent 1-based line numbers
        Range {
            start: start + 1,
            end: end + 1,
        }
    }

/// Parses the given Rust source code string to identify and categorize all top-level Rust items using the `rust-analyzer` AST.
/// It leverages `ra_ap_syntax::SourceFile::parse` to construct the syntax tree and then extracts all items, delegating the detailed analysis to `parse_all_rust_analyzer`.
/// The function provides a foundational step for understanding the structural elements of a Rust file.
///
/// # Arguments
///
/// * `src` - A string slice representing the Rust source code.
///
/// # Returns
///
/// A `Result` which is `Ok(HashMap<TextRange, AnalyzerRange>)` on success, containing a map of text ranges to `AnalyzerRange` structs for each identified top-level item, or an `ErrorHandling` enum if an error occurs during parsing.
    fn parse_result_items(src: &str) -> Result<HashMap<TextRange, AnalyzerRange>, ErrorHandling> {
        let parse = ra_ap_syntax::SourceFile::parse(src, ra_ap_ide::Edition::Edition2024);
        let items = parse
            .tree()
            .items()
            .collect::<Vec<ra_ap_syntax::ast::Item>>();
        parse_all_rust_analyzer(items)
    }

/// Parses a string slice representing Rust code and extracts a single `ObjectRange` corresponding to the primary code item found. This function is specifically designed to parse a snippet of Rust code (e.g., a single function or struct definition) and return its line range, type name, and identifier.
/// It leverages internal parsing utilities to first find all code objects and then focuses on the first one identified. This is particularly useful for analyzing isolated code blocks or changes.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) containing the Rust code to be parsed.
///
/// # Returns
///
/// A `Result<ObjectRange, ErrorHandling>` containing the `ObjectRange` of the first identified code item, or an `ErrorHandling` if no valid code object is found or parsing fails.
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling> {
        let analyzer: Vec<ObjectRange> = Self::parse_result_items(src)?
            .par_iter()
            .map(|val| {
                let line_ranges = Self::textrange_into_linerange(*val.0, src);
                ObjectRange {
                    line_ranges,
                    names: val.1.names.clone(),
                }
            })
            .collect::<Vec<ObjectRange>>();
        let visited: &ObjectRange = analyzer
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


}

/// Computes a vector of byte offsets for the start of each line in a given string slice. This function is a fundamental utility for converting between byte-based `TextRange` (used by syntax parsers) and human-readable line-based ranges.
/// It iterates through the input string, identifying newline characters to mark the beginning of subsequent lines. This is crucial for accurately mapping parsed syntax tree elements to their corresponding line numbers in a source file.
///
/// # Arguments
///
/// * `src` - A string slice (`&str`) representing the source code.
///
/// # Returns
///
/// A `Vec<usize>` where each element is the byte offset of the start of a line.
fn compute_line_starts(src: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Determines the 0-based line number corresponding to a given byte offset within a source text.
/// This helper function efficiently uses a pre-computed sorted list of line start offsets to find the correct line.
/// It performs a binary search to locate the line that the specified offset falls into.
///
/// # Arguments
///
/// * `offset` - The byte offset within the source text for which to find the line number.
/// * `line_starts` - A slice of `usize` values, where each value is the byte offset of the start of a line.
///
/// # Returns
///
/// A `usize` representing the 0-based line number corresponding to the given offset.
fn offset_to_line(offset: usize, line_starts: &[usize]) -> usize {
    match line_starts.binary_search(&offset) {
        Ok(line) => line,
        Err(next_line) => next_line - 1,
    }
}

/// Processes a vector of `rust-analyzer` AST items to extract their `TextRange` and identify their type and name.
/// It creates a `HashMap` where keys are `TextRange` and values are `AnalyzerRange` structs, categorizing each item like functions, structs, enums, impls, traits, and modules.
/// The function recursively descends into modules and `impl` blocks to find nested items, building a complete map of all recognized Rust constructs.
///
/// # Arguments
///
/// * `items` - A `Vec<ra_ap_syntax::ast::Item>` representing the parsed AST items from `rust-analyzer`.
///
/// # Returns
///
/// A `Result` which is `Ok(HashMap<TextRange, AnalyzerRange>)` on success, containing a map of text ranges to `AnalyzerRange` structs for each identified item, or an `ErrorHandling` enum if an error occurs during processing.
fn parse_all_rust_analyzer(
    items: Vec<ra_ap_syntax::ast::Item>,
) -> Result<HashMap<TextRange, AnalyzerRange>, ErrorHandling> {
    let mut analyzer: HashMap<TextRange, AnalyzerRange> = HashMap::new();
    for each in items {
        match each {
            ra_ap_syntax::ast::Item::Fn(f) => {
                let name = f.name();
                let range = f.syntax().text_range();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "fn".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "fn".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::Struct(s) => {
                let name = s.name();
                let range = s.syntax().text_range();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "struct".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "struct".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::Enum(e) => {
                let name = e.name();
                let range = e.syntax().text_range();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "enum".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "enum".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::Impl(i) => {
                let name_type = i.trait_();

                if let Some(name) = name_type {
                    let range = i.syntax().text_range();
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "impl".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    let range = i.syntax().text_range();
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "impl".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
                if let Some(val) = i.assoc_item_list() {
                    let items = val.assoc_items();
                    for each in items {
                        if let ra_ap_syntax::ast::AssocItem::Fn(f) = each {
                            let name = f.name();
                            let assoc_fn_range = f.syntax().text_range();
                            if let Some(name) = name {
                                analyzer.insert(
                                    assoc_fn_range,
                                    AnalyzerRange {
                                        range: assoc_fn_range,
                                        names: Name {
                                            type_name: "fn".to_string(),
                                            name: name.to_string(),
                                        },
                                    },
                                );
                            } else {
                                analyzer.insert(
                                    assoc_fn_range,
                                    AnalyzerRange {
                                        range: assoc_fn_range,
                                        names: Name {
                                            type_name: "fn".to_string(),
                                            name: "".to_string(),
                                        },
                                    },
                                );
                            }
                        }
                    }
                }
            }
            ra_ap_syntax::ast::Item::Trait(t) => {
                let name = t.name();
                let  range = t.syntax().text_range();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "trait".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "trait".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::TypeAlias(t) => {
                let name = t.name();
                let range = t.syntax().text_range();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "type_alias".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "type_alias".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::Use(u) => {
                let name = u.to_smolstr();
                let range = u.syntax().text_range();
                analyzer.insert(
                    range,
                    AnalyzerRange {
                        range,
                        names: Name {
                            type_name: "use".to_string(),
                            name: name.to_string(),
                        },
                    },
                );
            }
            ra_ap_syntax::ast::Item::MacroCall(m) => {
                let range = m.syntax().text_range();
                analyzer.insert(
                    range,
                    AnalyzerRange {
                        range,
                        names: Name {
                            type_name: "macro".to_string(),
                            name: "".to_string(),
                        },
                    },
                );
            }
            ra_ap_syntax::ast::Item::MacroRules(m) => {
                let range = m.syntax().text_range();
                let name = m.name();
                if let Some(name) = name {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "macro_rules".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "macro_rules".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            ra_ap_syntax::ast::Item::ExternBlock(e) => {
                let range = e.syntax().text_range();
                analyzer.insert(
                    range,
                    AnalyzerRange {
                        range,
                        names: Name {
                            type_name: "extern_block".to_string(),
                            name: "".to_string(),
                        },
                    },
                );
            }
            ra_ap_syntax::ast::Item::Module(m) => {
                let range = m.syntax().text_range();
                if let Some(name) = m.name() {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "mod".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "mod".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
                let items = m.item_list();
                if let Some(items) = items {
                    let module_items = items.items().collect::<Vec<ra_ap_syntax::ast::Item>>();
                    let k = parse_all_rust_analyzer(module_items)?;
                    analyzer.extend(k);
                }
            }
            ra_ap_syntax::ast::Item::TraitAlias(t) => {
                let range = t.syntax().text_range();
                if let Some(name) = t.name() {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "trait_alias".to_string(),
                                name: name.to_string(),
                            },
                        },
                    );
                } else {
                    analyzer.insert(
                        range,
                        AnalyzerRange {
                            range,
                            names: Name {
                                type_name: "trait_alias".to_string(),
                                name: "".to_string(),
                            },
                        },
                    );
                }
            }
            _ => (),
        }
    }

    Ok(analyzer)
}

/// Analyzes the input source code to identify and categorize various types of comments and lifetime indicators.
/// It processes line comments, block comments (both single and multi-line), and lifetime indicators, converting them into a structured `Vec<ObjectRange>`.
/// The function intelligently resolves multi-line block comments by combining their start and end markers into a single `ObjectRange` for simplified representation.
///
/// # Arguments
///
/// * `source_vector` - A string slice containing the source code to be analyzed.
///
/// # Returns
///
/// A `Result` which is `Ok(Vec<ObjectRange>)` on successful parsing, containing a vector of identified comment and lifetime ranges, or an `ErrorHandling` enum if an error occurs.
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
                            line_ranges: Range {
                                start: line_number,
                                end: line_number,
                            },
                            names: Name {
                                type_name: "CommentBlockSingeLine".to_string(),
                                name: "Comment".to_string(),
                            },
                        });
                    } else {
                        comment_vector.push(ObjectRange {
                            line_ranges: Range {
                                start: line_number,
                                end: 0,
                            },
                            names: Name {
                                type_name: "CommentBlockMultiLine".to_string(),
                                name: "Comment".to_string(),
                            },
                        });
                    };
                }
                TokenKind::Slash => {
                    comment_vector.push(ObjectRange {
                        line_ranges: Range {
                            start: line_number,
                            end: line_number,
                        },
                        names: Name {
                            type_name: "CommentBlockMultiLineEnd".to_string(),
                            name: "Refers to index - 1 (CommentBlockMultiLine)".to_string(),
                        },
                    });
                }
                TokenKind::LineComment => {
                    comment_vector.push(ObjectRange {
                        line_ranges: Range {
                            start: line_number,
                            end: line_number,
                        },
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
                        line_ranges: Range {
                            start: line_number,
                            end: line_number,
                        },
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
    if let Some(pos) = comment_vector
        .iter()
        .position(|obj| obj.names.type_name == multi_line)
    {
        found_position = pos;
    }
    if let Some(pos) = comment_vector
        .iter()
        .position(|obj| obj.names.type_name == multi_line_end)
    {
        comment_vector[found_position].line_ranges.end = comment_vector[pos].line_end();
        comment_vector.remove(pos);
    }

    Ok(comment_vector)
}

/// Removes all whitespace characters from the given input string.
/// This function iterates through each character of the input and constructs a new string containing only the non-whitespace characters.
///
/// # Arguments
///
/// * `s` - The input `String` from which whitespace should be removed.
///
/// # Returns
///
/// A `Result` which is `Ok(String)` containing the new string with all whitespace removed, or an `ErrorHandling` enum if an error occurs.
pub fn remove_whitespace(s: String) -> Result<String, ErrorHandling> {
    Ok(s.chars().filter(|c| !c.is_whitespace()).collect())
}
