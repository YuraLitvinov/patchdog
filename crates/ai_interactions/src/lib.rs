pub mod parse_json;

pub fn return_prompt() -> &'static str {
    "The provided data in function text is valid Rust code. Instruction: Locate each function in the structure, if context is present, use it at your disposal, elsewise proceed as is, generate /// comment for it and fill in the comment block. Return same structure with filled in comment block for each function. Dismiss."
}
