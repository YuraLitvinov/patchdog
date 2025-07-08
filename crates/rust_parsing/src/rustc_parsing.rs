use std::fs;

use rustc_lexer::TokenKind;
use rustc_lexer::tokenize;

pub fn comment_lexer(src: &str) {
    let source = match fs::read_to_string(src) {
        Ok(source) => source,
        Err(err) => format!("{:?}", err),
    };
    let tokenized = tokenize(&source);
    for each in tokenized {
        match each.kind {
            TokenKind::BlockComment { terminated } => {
                println!(
                    "terminated: {:?} kind: {:?} len: {:?}",
                    terminated, each.kind, each.len
                );
            }
            TokenKind::LineComment => {
                println!("kind: {:?} len: {:?}", each.kind, each.len);
            }

            TokenKind::Lifetime { starts_with_number } => {
                println!(
                    "Lifetime starts_with_number: {:?} kind: {:?} len: {:?}",
                    starts_with_number, each.kind, each.len
                );
            }
            TokenKind::Slash => {
                println!("kind: {:?} len: {:?}", each.kind, each.len);
            }
            _ => {}
        }
    }
}
