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
use std::path::{Path, PathBuf};
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
    fn find_module_file(
        base_path: PathBuf,
        mod_name: String,
    ) -> Result<Option<PathBuf>, ErrorHandling>;
    fn parse_rust_file(src: &Path) -> Result<Vec<ObjectRange>, ErrorHandling>;
    fn rust_item_parser(src: &str) -> Result<ObjectRange, ErrorHandling>;
    fn textrange_into_linerange(range: TextRange, src: &str) -> Range<usize>;
    fn parse_result_items(src: &str) -> Result<HashMap<TextRange, AnalyzerRange>, ErrorHandling>;
}

pub struct RustItemParser;

impl RustParser for RustItemParser {
    /// Parses a Rust source file from a given path into its Abstract Syntax Tree (AST) and extracts top-level Rust items.
    /// It reads the file content, then uses the `syn` crate to parse it into an AST representation.
    /// The function then visits the items in the AST to identify and collect information about each Rust item, such as functions, structs, or enums, along with their line ranges.
    ///
    /// # Arguments
    ///
    /// * `src` - A reference to a `Path` pointing to the Rust source file to be parsed.
    ///
    /// # Returns
    ///
    /// A `Result<Vec<ObjectRange>, ErrorHandling>` containing a vector of `ObjectRange` structs, each representing a parsed Rust item, or an error if file reading or AST parsing fails.
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

    fn parse_result_items(src: &str) -> Result<HashMap<TextRange, AnalyzerRange>, ErrorHandling> {
        let parse = ra_ap_syntax::SourceFile::parse(src, ra_ap_ide::Edition::Edition2024);
        let items = parse
            .tree()
            .items()
            .collect::<Vec<ra_ap_syntax::ast::Item>>();
        parse_all_rust_analyzer(items)
    }

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

fn compute_line_starts(src: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn offset_to_line(offset: usize, line_starts: &[usize]) -> usize {
    match line_starts.binary_search(&offset) {
        Ok(line) => line,
        Err(next_line) => next_line - 1,
    }
}

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
                    }
                }
            }
            ra_ap_syntax::ast::Item::Trait(t) => {
                let name = t.name();
                let range = t.syntax().text_range();
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
