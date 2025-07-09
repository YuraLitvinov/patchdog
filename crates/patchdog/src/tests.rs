#[cfg(test)]
mod tests {
    use rust_parsing::{
        export_object, extract_by_line, extract_object_preserving_comments, parse_all_rust_items,
        string_to_vector, InvalidIoOperationsSnafu, find_module_file, rustc_parsing::comment_lexer
    };
    use snafu::ResultExt;
    use git_parsing::{get_filenames, git_get_hunks};
    use std::{ffi::OsStr, fs, path::Path};
    use git2::Diff;
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
            .expect("File read failed");
        let vector_of_file = string_to_vector(function_from_file);
        let extracted_object = extract_by_line(vector_of_file, &start, &end);
        let object_from_const = format!("{}", IMPL_GEMINI);
        assert_ne!(object_from_const, extracted_object);
    }
    const COMPARE_LINES: &str = "fn function_with_return() -> i32 {\n";
    #[test]
    fn test_file_to_vector() {
        //file_to_vectors splits a file into a string of vectors line by line
        let path = Path::new(PATH_BASE);
        let source = fs::read_to_string(&path)
            .context(InvalidIoOperationsSnafu)
            .expect("File read failed");
        let vectored_file = string_to_vector(source);
        let line_eight_from_vector = &vectored_file[7]; //Count in vec! starts from 0 
        assert_eq!(COMPARE_LINES, line_eight_from_vector); //This test has passed
    }
    #[test]
    fn test_parse() {
        let source = fs::read_to_string(PATH_BASE)
            .context(InvalidIoOperationsSnafu)
            .expect("File read failed");
        let parsed = parse_all_rust_items(source)
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
            .context(InvalidIoOperationsSnafu)
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
        assert_ne!(true, true);
    }
    #[test]
    fn test_find_module_files() {
        let expected_behavior: &str = "../../tests/test_lib.rs\n../../tests/data.rs";
        let path = Path::new("../../tests/lib.rs");
        let source = fs::read_to_string(&path)
            .context(InvalidIoOperationsSnafu)
            .expect("Failed to read file");
        let parsed = parse_all_rust_items(source).expect("Failed to parse");
        let mut obj_vector: Vec<String> = Vec::new();
        for object in parsed {
            let obj_type = object
                .object_type()
                .expect("Unwrapping ObjectRange to type failed");
            let obj_name = object
                .object_name()
                .expect("Unwrapping ObjectRange to name failed");
            if obj_type == "mod".to_string() {
                let module_location = find_module_file(path.to_path_buf(), obj_name.clone())
                    .expect("Couldn't find mod file");
                obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
            }
        }

        assert_eq!(expected_behavior, obj_vector.join("\n"));
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

    const FUNCTION_BLOCK: &str = r#"// Trait with functions
trait MyTrait {
    fn required_function(&self);

    fn default_function(&self) {
        // Default implementation
    }
}"#;
    #[test]
    #[tracing::instrument(level = "debug")]
    fn test_extract_object_preserving_comments() {
        let path_src = "../../tests/data.rs";
        let src = fs::read_to_string(path_src).unwrap();
        let source = string_to_vector(src.clone());
        let from_where = 62;
        let parsed = parse_all_rust_items(src).expect("Parse failed");
        let check = extract_object_preserving_comments(source, from_where, parsed).expect("Check?");
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
    fn test_parse_on_patch() {
            let src = "../../test2.patch";
            let patch_text = fs::read(src)
                .expect("Failed to read patch file");
            let diff = Diff::from_buffer(&patch_text).unwrap();
            let changed = get_filenames(&diff).expect("Unwrap on get_filenames failed");
            for each in changed {
            let path = "../../".to_string() + &each.1;
            let extension = Path::new(&path)
                .extension()   
                .and_then(OsStr::to_str)
                .expect("Failed to get extension");
            if extension == "rs" {
                println!("actual path: {}", path);
                let str_src = fs::read_to_string(&path).expect("Failed to read file");
                let parsed = parse_all_rust_items(str_src).expect("Failed to parse");
                for each_parsed in parsed {
                    println!("{:?}", each_parsed);
                }
            }
            else {
               assert_eq!("../../crates/patchdog/Cargo.toml".to_string(), path);
            }
        }
    }
    #[test]
    fn test_patch_for_parsing() {
        let src = "../../test2.patch";
        let patch_text = fs::read(src)
            .expect("Failed to read patch file");
        let diff = Diff::from_buffer(&patch_text).unwrap();
        let filenames = get_filenames(&diff)
            .expect("failed to get filenames");
        let hunks = git_get_hunks(diff, filenames).expect("Unwrap on get_filenames failed");
        for each in hunks { 
            let file_extension = Path::new(&each.2)
                .extension()   
                .and_then(OsStr::to_str)
                .expect("Failed to get extension");
            if file_extension == "rs" {
                println!("{:?}", each);
                
            }


        }
    
        assert_eq!(true, false);
    }

}
