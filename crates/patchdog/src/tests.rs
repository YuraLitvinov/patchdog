mod tests {
    use crate::cli::cherrypick_response;
    use gemini::request_preparation::RawResponse;
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu};
    use rust_parsing::file_parsing::{FileExtractor, Files, REGEX};
    use rust_parsing::object_range::Name;
    use rust_parsing::{
        ObjectRange,
        rust_parser::{RustItemParser, RustParser},
    };
    use snafu::ResultExt;
    use std::collections::HashMap;
    use std::{env, fs, path::Path};
    const PATH_BASE: &str = "../../tests/data.rs";

    #[test]
    fn test_parser() {
        let src = fs::read_to_string(PATH_BASE).unwrap();
        let analyzer = RustItemParser::parse_result_items(&src)
            .unwrap()
            .into_iter()
            .map(|val| {
                let range = RustItemParser::textrange_into_linerange(val.0.clone(), &src);
                (range.to_owned(), val.1.clone())
            })
            .collect::<HashMap<std::ops::Range<usize>, rust_parsing::rust_parser::AnalyzerRange>>();
        let parser = RustItemParser::parse_all_rust_items(&src)
            .unwrap()
            .iter()
            .filter_map(|val| {
                if val.names.type_name == "LifetimeIndicator"
                    || val.names.type_name == "LineComment"
                {
                    None
                } else {
                    Some(val.clone())
                }
            })
            .collect::<Vec<ObjectRange>>();

        println!("{}", parser.len());
        println!("{}", analyzer.len());

        assert_eq!(parser.len(), analyzer.len());
    }

    /// Tests the `parse_all_rust_items` function by reading a Rust file from `PATH_BASE` and parsing its contents.
    /// It then iterates through the parsed objects, printing `impl` objects.
    /// The test asserts `true` for `true`, which is a placeholder and does not validate the parsing logic itself.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The file at `PATH_BASE` cannot be read.
    /// - `parse_all_rust_items` fails to parse the source.
    #[test]
    fn test_parse() {
        let source = fs::read_to_string(PATH_BASE).expect("File read failed");
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Parsing failed");
        for object in parsed {
            let obj_type = object.names.type_name.clone();
            if obj_type == "impl".to_string() {
                println!("{:?}", object);
            }
        }

        assert_eq!(true, true);
    }

    /// Tests the `parse_all_rust_items` function to find all function definitions in a Rust file.
    /// It reads a Rust file from `PATH_BASE`, parses its contents, and then iterates through the parsed objects.
    /// If an object is identified as a function (`"fn"`), its debug representation is printed.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The file at `PATH_BASE` cannot be read.
    /// - `parse_all_rust_items` fails to parse the source.
    #[test]
    fn find_all_fn() {
        let source = fs::read_to_string(PATH_BASE)
            .context(InvalidIoOperationsSnafu { path: PATH_BASE })
            .expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
        for object in parsed {
            let obj_type = object.names.type_name.clone();
            if obj_type == "fn".to_string() {
                println!("{:?}", object);
            }
        }
    }

    /// Tests the `find_module_file` function to ensure it correctly locates module files.
    /// It reads a test Rust file (`../../tests/lib.rs`), parses it to find module declarations.
    /// For each module found, it attempts to locate its corresponding file using `find_module_file`.
    /// The collected module file paths are then joined into a single string and compared against an `expected_behavior` string.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The test file cannot be read.
    /// - Parsing of the test file fails.
    /// - `find_module_file` fails to locate a declared module file.
    /// - The assertion `assert_eq!` fails, indicating a mismatch between expected and actual module file paths.
    #[test]
    fn test_find_module_files() {
        let expected_behavior: &str = "../../tests/test_lib.rs\n../../tests/data.rs";
        let path = Path::new("../../tests/lib.rs");
        let source = fs::read_to_string(&path)
            .context(InvalidIoOperationsSnafu { path: path })
            .expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
        let mut obj_vector: Vec<String> = Vec::new();
        for object in parsed {
            let obj_type = object.names.type_name;
            let obj_name = object.names.name;
            if obj_type == "mod".to_string() {
                let module_location =
                    RustItemParser::find_module_file(path.to_path_buf(), obj_name.to_owned())
                        .expect("Couldn't find mod file");
                obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
            }
        }

        assert_eq!(expected_behavior, obj_vector.join("\n"));
    }

    /// Tests the ability to read and construct file paths for arguments.
    /// It retrieves the current working directory, manipulates it to construct a mock `path_to_patch`,
    /// and then asserts `true` for `true`, which is a placeholder and does not perform active path validation.
    /// The commented-out `assert_eq!` suggests an original intention to validate the constructed path.
    ///
    /// # Panics
    ///
    /// This test will panic if `env::current_dir()` fails to retrieve the current directory.
    #[test]
    fn test_read_argument() {
        let mut path = env::current_dir().expect("couldn't get path");
        path.pop();
        path.pop();
        let _path_to_patch = path.join("patch.patch");
        /*
        assert_eq!(
            path_to_patch,
            Path::new("/home/runner/work/patchdog/patchdog/patch.patch")
        );
        */
        assert_eq!(true, true);
    }

    /// A placeholder test function intended for covering scenarios with empty objects.
    /// The commented-out code suggests an initial design involving `Name`, `LineRange`, and `ObjectRange` structs,
    /// but no actual test logic is implemented.
    ///
    /// This test currently does nothing.
    #[test]
    fn test_cover_empty_object() {
        /*
        let mut name: Vec<Name> = Vec::new();
        let mut ranges: Vec<LineRange> = Vec::new();
        let mut _objectrange: Vec<ObjectRange> = Vec::new();
        */
    }

    /// Tests the `parse_all_rust_items` function's ability to identify and parse comments within a Rust file.
    /// It reads a specific Rust file (`../../crates/patchdog/src/binding.rs`), parses its content,
    /// and then prints the debug representation of each parsed object, including comments.
    /// The comment `//block is of 94 symbols length` appears to be a note about expected content for testing.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The specified file cannot be read.
    /// - `parse_all_rust_items` fails to parse the file content.
    #[test]
    fn find_comments() {
        //block is of 94 symbols length
        let file = fs::read_to_string("../../crates/patchdog/src/binding.rs").expect("err on 159");
        let parsed = RustItemParser::parse_all_rust_items(&file).expect("err");
        for each in parsed {
            println!("{:?}", each);
        }
    }
    const _JSON: &str = r#"{
  "files": [
    {
      "filename": "/home/yurii-sama/patchdog/crates/gemini/src/lib.rs",
      "types": {
        "fn": [
          {
            "name": "req_res",
            "comment": "/// Sends the given file content to the Gemini model and returns the generated response.\n/// \n/// # Arguments\n/// \n/// * `file_content` - A string representation of the file to send to the model.\n/// \n/// # Returns\n/// \n/// A `Result` containing the model's response as a `String`, or an error if the request fails."
          }
        ],
        "impl": [
          {
            "name": "GoogleGemini",
            "comment": "/// Implementation of the Gemini API interaction logic."
          }
        ],
        "const": [],
        "struct": [],
        "enum": [],
        "trait": [],
        "type": []
      }
}]}"#;

    /// A placeholder test function for validating the output from an AI agent.
    /// The commented-out code outlines a plan to:
    /// 1. Assess if the AI agent's JSON output is valid.
    /// 2. Implement recursive calls to retry if the output is invalid or missing required fields (filename, function name).
    /// The test currently returns `Ok(())` without executing any of the assessment logic.
    ///
    /// This test currently does nothing.
    #[test]
    fn test_agent_out() -> Result<(), ErrorBinding> {
        /*
        1. We need to assess whether the JSON given by the AI Agent is valid first-hand. If it's not, then we recursively call
        function to run again and again until there is a proper response.
        serde_json(json) is_err(): retry
        2. Filename is_err(): retry
        3. search function is.err(): retry
        //call_agent - using it as mock for calling AI Agent again

        let assessed = assess_correct_output(JSON.to_string())?;
        let expected = Assess {
            filename: "/home/yurii-sama/patchdog/crates/gemini/src/lib.rs".to_string(),
            names: vec!["req_res".to_string()],
        };
        assert_eq!(assessed[0], expected);
        */
        Ok(())
    }

    /// Tests the `cherrypick_response` function's ability to extract specific JSON objects using a regex.
    /// It reads a test JSON file (`../../tests/response_regex.json`), applies `cherrypick_response` to it,
    /// and asserts that the number of extracted `RawResponse` objects is `3`.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The test JSON file cannot be read.
    /// - `cherrypick_response` returns an error.
    /// - The assertion `assert_eq!` fails, meaning the number of extracted responses is not `3`.
    #[test]
    fn test_regex() {
        let test = fs::read_to_string(Path::new("../../tests/response_regex.json")).unwrap();
        let assess_size = cherrypick_response(&test).unwrap();
        assert_eq!(assess_size.len(), 3);
    }

    /// Tests various parsing scenarios for agent responses, including handling of malformed JSON.
    /// It uses a predefined regex (`REGEX`) to cherry-pick responses and also attempts direct JSON deserialization.
    /// If direct deserialization fails (as expected for certain test cases), it performs a fallback mechanism by manipulating the response string (removing first and last lines, joining) and re-attempts deserialization.
    /// Assertions check the lengths of the parsed results from both methods.
    ///
    /// # Returns
    ///
    /// An `Ok(())` on successful completion of the tests.
    /// An `Err(ErrorHandling)` if any file operation or parsing (including regex or JSON) fails unexpectedly.
    #[test]
    fn test_response() -> Result<(), ErrorHandling> {
        let re = Regex::new(REGEX).unwrap();
        let test = fs::read_to_string(Path::new("../../tests/response_regex.json")).unwrap();
        let mut assess_size = vec![];
        for cap in re.captures_iter(&test) {
            let a = cap.get(0).unwrap().as_str();
            let to_struct = serde_json::from_str::<RawResponse>(a).unwrap();
            assess_size.push(to_struct);
        }
        match serde_json::from_str::<Vec<RawResponse>>(&test) {
            Ok(ok) => {
                println!("{} = {}", ok.len(), assess_size.len());
            }
            Err(_) => {
                let as_vec = FileExtractor::string_to_vector(&test);
                let a = &as_vec[1..as_vec.len() - 1].join("\n");
                let to_struct = serde_json::from_str::<Vec<RawResponse>>(a).unwrap();
                assert_eq!(assess_size.len(), 3);
                assert_eq!(to_struct.len(), 58);
            }
        }
        Ok(())
    }

    #[test]
    fn test_match() {
        let names = Name {
            type_name: "fn".to_string(),
            name: "request_manager".to_string(),
        };
        let _input = ObjectRange {
            line_ranges: 395..429,
            names,
        };

        //match_context(context: Vec<(PathBuf, ObjectRange)>) -> HashMap<String, PathObject>
        //let deserial_in = serde_json::from_str::<ObjectRange>(input).expect("err failed to parse from json ObjectRange");
        //let deserial_out = serde_json::from_str::<PathObject>(output).expect("err failed to parse from json PathObject");
    }
}
