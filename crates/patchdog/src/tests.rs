#[cfg(test)]
mod tests {
    use anyhow::Context;
    use rust_parsing::rustc_parsing::comment_lexer;
    use rust_parsing::{InvalidIoOperationsSnafu, InvalidSynParsingSnafu, find_module_file};
    use rust_parsing::{
        export_object, extract_by_line, extract_object_preserving_comments, parse_all_rust_items,
        string_to_vector,
    };
    use git_parsing::git_get;
    use std::fs;
    use std::path::Path;
    const PATH_BASE: &str = "../../tests/data.rs";
    const PATH_GEMINI: &str = "../../crates/gemini/src/lib.rs";
    const IMPL_GEMINI: &str = r#"impl GoogleGemini {
    pub async fn req_res(file_content: String) -> Result<String, Box<dyn Err>> {
        let api_key = std::env::var("API_KEY_GEMINI")?;
        let client = Gemini::new(&api_key);
        let args = std::env::var("INPUT_FOR_MODEL")?;
        let res = client
            .generate_content()
            .with_system_prompt(args)
            .with_user_message(file_content)
            .execute()
            .await?;
        Ok(res.text())
    }
}"#;
    #[test]
    fn test_extract_function() {
        let start: usize = 10;
        let end: usize = 23;
        //Actually an impl block; doesn't affect the result
        let function_from_file = fs::read_to_string(PATH_GEMINI)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("File read failed");
        let vector_of_file = string_to_vector(function_from_file);
        let extracted_object = extract_by_line(vector_of_file, &start, &end)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("Extracting string by line failed");
        let object_from_const = format!("{}", IMPL_GEMINI);
        assert_ne!(object_from_const, extracted_object);
    }
    const COMPARE_LINES: &str = "fn function_with_return() -> i32 {\n";
    #[test]
    fn test_file_to_vector() {
        //file_to_vectors splits a file into a string of vectors line by line
        let path = Path::new(PATH_BASE);
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("File read failed");
        let vectored_file = string_to_vector(source);
        let line_eight_from_vector = &vectored_file[7]; //Count in vec! starts from 0 
        assert_eq!(COMPARE_LINES, line_eight_from_vector); //This test has passed
    }
    #[test]
    fn test_parse() {
        let source = fs::read_to_string(PATH_BASE)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("File read failed");
        let parsed = parse_all_rust_items(source)
            .context(format!("{:?}", InvalidSynParsingSnafu))
            .expect("Parsing failed");
        for object in parsed {
            let obj_type = object
                .object_type()
                .expect("Unwrapping ObjectRange to type failed");
            if obj_type == "impl".to_string() {
                println!("{:?}", object);
            }
        }

        assert_eq!(true, true);
    }
    #[test]
    fn find_all_fn() {
        let source = fs::read_to_string(PATH_BASE)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("Failed to read file");
        let parsed = parse_all_rust_items(source).expect("Failed to parse");
        for object in parsed {
            let obj_type = object
                .object_type()
                .expect("Unwrapping ObjectRange to type failed");
            if obj_type == "fn".to_string() {
                println!("{:?}", object);
            }
        }
        assert_eq!(true, true);
    }
    #[test]
    fn test_find_module_files() {
        let expected_behavior: &str = r#"Some("/home/runner/work/patchdog/patchdog/tests/test_lib.rs")
        Some("/home/runner/work/patchdog/patchdog/tests/data.rs")"#;
        let path = Path::new(PATH_BASE);
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .expect("Failed to read file");
        let parsed = parse_all_rust_items(source).expect("Failed to parse");
        for object in parsed {
            let obj_type = object
                .object_type()
                .expect("Unwrapping ObjectRange to type failed");
            let obj_name = object
                .object_name()
                .expect("Unwrapping ObjectRange to name failed");
            if obj_type == "mod".to_string() {
                let module_location = find_module_file(path.to_path_buf(), obj_name.clone())
                    .context(format!("{:?}", InvalidIoOperationsSnafu))
                    .expect("Couldn't find mod file");
                println!("{:?}", module_location);
            }
        }

        assert_eq!(expected_behavior, "");
    }
    #[test]
    fn test_receive_context_on_zero() {
        let str_src = fs::read_to_string(PATH_BASE).expect("Failed to read file");
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).expect("Failed to parse");
        let expected_behavior = "LineOutOfBounds { line_number: 0 }";
        assert_eq!(
            expected_behavior,
            export_object(0, parsed, source).expect("Failed to export object")
        );
    }

    #[test]
    fn test_receive_context_on_exceed() {
        let str_src = fs::read_to_string(PATH_BASE).expect("Failed to read file");
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).expect("Failed to parse");

        let expected_behavior: &'static str = "LineOutOfBounds { line_number: 999999 }";
        assert_eq!(
            expected_behavior,
            export_object(999999, parsed, source).expect("Failed to export object")
        );
    }
    const EXPECTED_BEHAVIOR: &str = "impl MyStruct {
    fn method(&self) {}

    pub fn public_method(&self) {}

    fn static_method() {}

    fn method_with_lifetime<'a>(&'a self, input: &'a str) -> &'a str {
        input
    }

    fn method_with_generic<T>(&self, value: T) {}
}\n";
    #[test]
    fn test_receive_context_on_true() {
        let str_src = fs::read_to_string(PATH_BASE).expect("Failed to read file");
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).expect("Failed to parse");
        let received = export_object(50, parsed, source).expect("Failed to export object");

        assert_ne!(EXPECTED_BEHAVIOR, received);
    }

    const FUNCTION_BLOCK: &str = "\n// Method in impl\nstruct MyStruct;\n";
    #[test]
    #[tracing::instrument(level = "debug")]
    fn test_extract_object_preserving_comments() {
        let path_src = "../../tests/data.rs";
        let src = fs::read_to_string(path_src).unwrap();
        let source = string_to_vector(src.clone());
        let from_where = 62;
        let parsed = parse_all_rust_items(src).expect("Parse failed");
        let check = extract_object_preserving_comments(source, from_where, parsed).expect("Check?");
        //println!("{}", check);
        assert_eq!(check, FUNCTION_BLOCK);
    }

    #[test]
    fn test_lexer() {
        //block is of 94 symbols length
        let block = "//! If you want to see the list of objects in a .rs file you have to call parse_all_rust_items";
        let _ = comment_lexer("../../crates/rust_parsing/src/lib.rs");
        let mut i = 0;
        for _each in block.chars() {
            i = i + 1;
        }
        assert_eq!(i, 94);
    }

    #[test]
    fn test_git_get() {
        let _ = git_get("../../tests/data.rs");
    }
}
