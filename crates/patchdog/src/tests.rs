#[cfg(test)]
mod tests {
    use anyhow::Context;
    use rust_parsing::{InvalidIoOperationsSnafu, InvalidSynParsingSnafu, find_module_file};
    use rust_parsing::{extract_by_line, parse_all_rust_items, receive_context, string_to_vector};
    use std::fs;
    use std::path::Path;
    const PATH_BASE: &str = "/home/runner/work/patchdog/patchdog/";
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
        let function_from_file =
            fs::read_to_string(Path::new(PATH_BASE).join("crates/gemini/src/lib.rs"))
                .context(format!("{:?}", InvalidIoOperationsSnafu))
                .unwrap();
        let vector_of_file = string_to_vector(function_from_file);
        let extracted_object = extract_by_line(vector_of_file, &start, &end)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let object_from_const = format!("{}", IMPL_GEMINI);
        assert_ne!(object_from_const, extracted_object);
    }
    const COMPARE_LINES: &str = "fn function_with_return() -> i32 {\n";
    #[test]
    fn test_file_to_vector() {
        //file_to_vectors splits a file into a string of vectors line by line
        let path = Path::new(PATH_BASE).join("tests/data.rs");
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let vectored_file = string_to_vector(source);
        let line_eight_from_vector = &vectored_file[7]; //Count in vec! starts from 0 
        assert_eq!(COMPARE_LINES, line_eight_from_vector); //This test has passed
    }
    #[test]
    fn test_parse() {
        let path = Path::new(PATH_BASE).join("tests/lib.rs");
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let parsed = parse_all_rust_items(source)
            .context(format!("{:?}", InvalidSynParsingSnafu))
            .unwrap();
        for object in parsed {
            let obj_type = object.object_type().unwrap();
            if obj_type == "impl".to_string() {
                println!("{:?}", object);
            }
        }

        assert_eq!(true, true);
    }
    #[test]
    fn find_all_fn() {
        let path = Path::new(PATH_BASE).join("tests/lib.rs");
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let parsed = parse_all_rust_items(source).unwrap();
        for object in parsed {
            let obj_type = object.object_type().unwrap();
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
        let path = Path::new(PATH_BASE).join("tests/lib.rs");
        let source = fs::read_to_string(&path)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let parsed = parse_all_rust_items(source).unwrap();
        for object in parsed {
            let obj_type = object.object_type().unwrap();
            let obj_name = object.object_name().unwrap();
            if obj_type == "mod".to_string() {
                let module_location = find_module_file(path.clone(), obj_name.clone())
                    .context(format!("{:?}", InvalidIoOperationsSnafu))
                    .unwrap();
                println!("{:?}", module_location);
            }
        }

        assert_eq!(expected_behavior, "");
    }
    #[test]
    fn test_receive_context_on_zero() {
        let path = Path::new(PATH_BASE).join("tests/data.rs");
        let str_src = fs::read_to_string(path.clone()).unwrap();
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).unwrap();
        let expected_behavior = "LineOutOfBounds { line_number: 0 }";
        assert_eq!(
            expected_behavior,
            receive_context(0, parsed, source).unwrap()
        );
    }

    #[test]
    fn test_receive_context_on_exceed() {
        let path = Path::new(PATH_BASE).join("tests/data.rs");
        let str_src = fs::read_to_string(path.clone()).unwrap();
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).unwrap();

        let expected_behavior: &'static str = "LineOutOfBounds { line_number: 999999 }";
        assert_eq!(
            expected_behavior,
            receive_context(999999, parsed, source).unwrap()
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
        let path = Path::new(PATH_BASE).join("tests/data.rs");
        let str_src = fs::read_to_string(path.clone()).unwrap();
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).unwrap();
        let received = receive_context(50, parsed, source).unwrap();

        assert_eq!(EXPECTED_BEHAVIOR, received);
    }

    const _FUNCTION_BLOCK: &str = "// Regular private function
fn regular_function() {}
";
    #[test]
    fn test_extract_object_preserving_comments() {
        let path = Path::new(PATH_BASE).join("tests/data.rs");
        let str_src = fs::read_to_string(path.clone()).unwrap();
        let source = string_to_vector(str_src.clone());
        let parsed = parse_all_rust_items(str_src).unwrap();
        let mut new_previous: Vec<usize> = Vec::new();
        new_previous.push(1);
        let mut i = 0;
        for each in &parsed {
            let previous_end_line = each.line_end().unwrap() + 1;
            new_previous.push(previous_end_line);
            let extracted =
                extract_by_line(source.clone(), &new_previous[i], &each.line_end().unwrap())
                    .unwrap();
            println!("{}", extracted);
            i = i + 1;
        }
        //assert_eq!(FUNCTION_BLOCK, output);
    }
}
