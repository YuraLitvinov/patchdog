use rustc_lexer::TokenKind;
use rustc_lexer::tokenize;
///Comment lexer parses a single string, requiring it to be into iterator of vecstrings created by string_to_vector
///This allows to justify whether a line contains a single-line comment or not
pub fn comment_lexer(source: String, line_number: usize) {
    let tokenized = tokenize(&source);
    for each in tokenized {
        match each.kind {
            TokenKind::BlockComment { terminated } => {
                println!(
                    "line {} terminated: {:?} kind: {:?} len: {:?}",
                    line_number, terminated, each.kind, each.len
                );
            }
            TokenKind::LineComment => {
                println!("line: {} kind: {:?} len: {:?}", line_number, each.kind, each.len);
            }

            TokenKind::Lifetime { starts_with_number } => {
                println!(
                    "line {} Lifetime starts_with_number: {:?} kind: {:?} len: {:?}",
                    line_number, starts_with_number, each.kind, each.len
                );
            }
            TokenKind::Slash => {
                println!("line {} kind: {:?} len: {:?}", line_number, each.kind, each.len);
            }
            _ => {}
        }
    }
}
