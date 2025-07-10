mod tests {
    use git_parsing::{
        get_filenames, git_get_hunks, match_patch_with_parse, read_non_repeting_functions,
    };
    use git2::Diff;
    use rust_parsing::rust_parser::{RustItemParser, RustParser};
    use rust_parsing::error::InvalidIoOperationsSnafu;
    use rust_parsing::comment_lexer;

    use rust_parsing::file_parsing::{FileExtractor, Files};

    use snafu::ResultExt;
    use std::{ffi::OsStr, fs, path::Path};
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
        let function_from_file = fs::read_to_string(PATH_GEMINI).expect("File read failed");
        let vector_of_file = FileExtractor::string_to_vector(&function_from_file);
        let extracted_object = FileExtractor::extract_by_line(&vector_of_file, &start, &end);
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
        let vectored_file = FileExtractor::string_to_vector(&source);
        let line_eight_from_vector = &vectored_file[7]; //Count in vec! starts from 0 
        assert_eq!(COMPARE_LINES, line_eight_from_vector); //This test has passed
    }
    #[test]
    fn test_parse() {
        let source = fs::read_to_string(PATH_BASE)
            .context(InvalidIoOperationsSnafu)
            .expect("File read failed");
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Parsing failed");
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
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
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
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
        let mut obj_vector: Vec<String> = Vec::new();
        for object in parsed {
            let obj_type = object
                .object_type()
                .expect("Unwrapping ObjectRange to type failed");
            let obj_name = object
                .object_name()
                .expect("Unwrapping ObjectRange to name failed");
            if obj_type == "mod".to_string() {
                let module_location =
                    RustItemParser::find_module_file(path.to_path_buf(), obj_name.clone())
                        .expect("Couldn't find mod file");
                obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
            }
        }

        assert_eq!(expected_behavior, obj_vector.join("\n"));
    }
    #[test]
    fn test_receive_context_on_zero() {
        let str_src = fs::read_to_string(PATH_BASE).expect("Failed to read file");
        let source = FileExtractor::string_to_vector(&str_src);
        let parsed = RustItemParser::parse_all_rust_items(&str_src).expect("Failed to parse");
        let expected_behavior = "LineOutOfBounds { line_number: 0 }";
        assert_eq!(
            expected_behavior,
            FileExtractor::export_object(0, &parsed, &source).expect("Failed to export object")
        );
    }
    #[test]
    fn test_receive_context_on_exceed() {
        let str_src = fs::read_to_string(PATH_BASE).expect("Failed to read file");
        let source = FileExtractor::string_to_vector(&str_src);
        let parsed = RustItemParser::parse_all_rust_items(&str_src).expect("Failed to parse");

        let expected_behavior: &'static str = "LineOutOfBounds { line_number: 999999 }";
        assert_eq!(
            expected_behavior,
            FileExtractor::export_object(999999, &parsed, &source)
                .expect("Failed to export object")
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
        let source = FileExtractor::string_to_vector(&str_src);
        let parsed = RustItemParser::parse_all_rust_items(&str_src).expect("Failed to parse");
        let received =
            FileExtractor::export_object(50, &parsed, &source).expect("Failed to export object");

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
        let source = FileExtractor::string_to_vector(&src);
        let from_where = 62;
        let parsed = RustItemParser::parse_all_rust_items(&src).expect("Parse failed");
        let check = FileExtractor::extract_object_preserving_comments(source, from_where, parsed)
            .expect("Check?");
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
        let patch_text = fs::read(src).expect("Failed to read patch file");
        let diff = Diff::from_buffer(&patch_text).unwrap();
        let changed = get_filenames(&diff).expect("Unwrap on get_filenames failed");
        for each in changed {
            let path = "../../".to_string() + &each;
            let extension = Path::new(&path)
                .extension()
                .and_then(OsStr::to_str)
                .expect("Failed to get extension");
            if extension == "rs" {
                println!("actual path: {}", path);
                let str_src = fs::read_to_string(&path).expect("Failed to read file");
                let parsed =
                    RustItemParser::parse_all_rust_items(&str_src).expect("Failed to parse");
                for each_parsed in parsed {
                    println!("{:?}", each_parsed);
                }
            } else {
                assert_eq!("../../crates/patchdog/Cargo/toml".to_string(), path);
            }
        }
    }

    #[test]
    fn test_combine_git() {
        let src = "../../patchfromlatest.patch";
        let patch_text = fs::read(src).expect("Failed to read patch file");
        let read = read_non_repeting_functions(&patch_text, "../../").expect("Failed to read");
        for each in read {
            println!("{:?}", each);
            let file_src = fs::read_to_string(each).expect("failed to read");
            let parsed = RustItemParser::parse_all_rust_items(&file_src).expect("failed parse");
            for each in parsed {
                if each.object_type().unwrap() == "fn" {
                    println!("{:?}", each);
                }
            }
        }
        assert_eq!(true, false);
    }
    #[test]
    fn test_match_patch_with_parse() {
        let src = "/home/yurii-sama/Desktop/patchdog/patch.patch";
        let patch_text = fs::read(src).expect("Failed to read patch file");
        let read = read_non_repeting_functions(&patch_text, "/home/yurii-sama/Desktop/patchdog")
            .expect("Failed to read");
        let diff = Diff::from_buffer(&patch_text).unwrap();
        let changed = get_filenames(&diff).expect("Unwrap on get_filenames failed");
        let mut hunks = git_get_hunks(diff, changed).expect("Error?");

        hunks.sort_by(|a, b| a.get_line().cmp(&b.get_line()));
        for read in read {
            for each in hunks.clone().into_iter() {
                println!("{:?}", each.filename());
                let path = "/home/yurii-sama/Desktop/patchdog/".to_string() + &each.filename();
                let file = fs::read_to_string(&path).expect("failed to read");
                let file_vector = FileExtractor::string_to_vector(&file);
                let parsed = RustItemParser::parse_all_rust_items(&file).expect("failed to parse");
                let what_change_occured = match each.get_change() {
                    "Add" => {
                        println!("Added: \n {:?}", &read);
                        FileExtractor::export_object(each.get_line(), &parsed, &file_vector)
                            .unwrap()
                    }

                    "Remove" => {
                        println!("Removed line number: {} \n{:?} ", each.get_line(), &read);
                        "".to_string()
                    }
                    "Modify" => {
                        println!("Modified: \n {}", &each.filename());
                        FileExtractor::export_object(each.get_line(), &parsed, &file_vector)
                            .expect("Error on Modify")
                    }
                    _ => "".to_string(),
                };
                println!("{}", what_change_occured);
            }
        }

        assert_eq!(true, false);
    }
    #[test]
    fn test_export_object() {
        let file =
            fs::read_to_string("/home/yurii-sama/Desktop/patchdog/crates/git_parsing/src/lib.rs")
                .expect("failed to read");
        let parsed = RustItemParser::parse_all_rust_items(&file).expect("Failed to parse");
        for each in &parsed {
            println!("{:?}", each);
        }
        let check = FileExtractor::check_for_not_comment(&parsed, 45);
        println!("is there comment: {:?}", check);
        let file_vector = FileExtractor::string_to_vector(&file);
        let what_change_occured =
            FileExtractor::export_object(45, &parsed, &file_vector).expect("Error on Modify");

        assert_eq!(what_change_occured, "");
    }

    #[test]
    fn testing_required() {
        fn _match_patch_with_parse(src: &str, relative_path: &str) {
            let path = format!("{}{}", relative_path, src);
            let patch_text = fs::read(&path).expect("Failed to read patch file");
            let read =
                read_non_repeting_functions(&patch_text, &relative_path).expect("Failed to read");
            let diff = Diff::from_buffer(&patch_text).unwrap();
            let changed = get_filenames(&diff).expect("Unwrap on get_filenames failed");
            let mut hunks = git_get_hunks(diff, changed).expect("Error?");

            hunks.sort_by(|a, b| a.filename().cmp(&b.filename()));
            for read in &read {
                println!("READ: {:?}", &read);
                for each in hunks.clone().into_iter() {
                    let path = read;
                    let file = fs::read_to_string(&path).expect("failed to read");
                    let file_vector = FileExtractor::string_to_vector(&file);
                    let parsed =
                        RustItemParser::parse_all_rust_items(&file).expect("failed to parse");
                    let what_change_occured = match each.get_change() {
                        "Add" => {
                            println!("Added: \n {:?}", &each);
                            let exported_object = FileExtractor::export_object(
                                each.get_line(),
                                &parsed,
                                &file_vector,
                            )
                            .unwrap();
                            if FileExtractor::check_for_not_comment(&parsed, each.get_line())
                                .unwrap()
                                || exported_object.trim().is_empty()
                            {
                                println!("SKIPPED ADD AT :{} {}", exported_object, each.get_line());
                                continue;
                            }
                            exported_object
                        }

                        "Remove" => {
                            println!("Removed line number:\n{:?}", &each.filename());
                            "".to_string()
                        }
                        "Modify" => {
                            println!("Modified: \n {:?}", &each);
                            let exported_object = FileExtractor::export_object(
                                each.get_line(),
                                &parsed,
                                &file_vector,
                            )
                            .unwrap();
                            if FileExtractor::check_for_not_comment(&parsed, each.get_line())
                                .unwrap()
                                || exported_object.trim().is_empty()
                            {
                                println!(
                                    "SKIPPED MODIFY AT :{} {}",
                                    exported_object,
                                    each.get_line()
                                );
                                continue;
                            }
                            exported_object
                        }
                        _ => "".to_string(),
                    };
                    if what_change_occured.trim().is_empty() {
                        continue;
                    }
                    println!("{}", what_change_occured);
                }
            }
        }
    }

    #[test]
    fn test_quantity() {
        let patch_text = fs::read("/home/yurii-sama/Desktop/patchdog/patch.patch")
            .expect("Failed to read patch file");
        // let mut vec_of_exports: Vec<String> = Vec::new();
        let matched = match_patch_with_parse("", &patch_text).unwrap();
        for change_line in matched {
            if change_line.quantity == 1 {
                //println!("{:?}", change_line.change_at_hunk);

                let path = "/home/yurii-sama/Desktop/patchdog/".to_string()
                    + &change_line.change_at_hunk.filename();
                let file = fs::read_to_string(path).expect("Failed to read file");
                let parsed = RustItemParser::parse_all_rust_items(&file).expect("Failed to parse");
                println!("{}", change_line.change_at_hunk.filename);

                for each in parsed {
                    println!("{:?}", each);
                }
            }
        }

        assert_eq!(true, false);
    }
}
