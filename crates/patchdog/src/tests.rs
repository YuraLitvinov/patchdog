mod tests {
    use crate::binding;
    use crate::cli::cherrypick_response;
    use gemini::gemini::{RawResponse, SingleFunctionData};
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu, SerdeSnafu};
    use rust_parsing::file_parsing::{FileExtractor, Files, REGEX};
    use rust_parsing::rust_parser::{RustItemParser, RustParser};
    use snafu::ResultExt;
    use std::env;
    use std::io::Write;
    use std::process::{Command, id};
    use std::{fs, path::Path};
    use syn::{ExprTry, Item};
    use tempfile::NamedTempFile;
    const PATH_BASE: &str = "../../tests/data.rs";

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
            .context(InvalidIoOperationsSnafu)
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
            .context(InvalidIoOperationsSnafu)
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

/// Tests the `get_patch_data` function by generating a Git patch and processing it.
/// It retrieves the current working directory, generates a Git patch for the last commit using `git format-patch --stdout -1 HEAD`,
/// writes this patch to a temporary file, and then calls `binding::get_patch_data` to process it.
/// The extracted patch data is then printed to the console.
///
/// # Panics
///
/// This test will panic if:
/// - `env::current_dir()` fails to retrieve the current directory.
/// - The `git` command fails to execute or returns an error.
/// - A temporary file cannot be created or written to.
/// - `binding::get_patch_data` fails to process the patch.
    #[test]
    fn test_read_patch() {
        let mut path = env::current_dir()
            .context(InvalidIoOperationsSnafu)
            .expect("couldn't get current dir");
        path.pop();
        path.pop();
        let output = Command::new("git")
            .args(["format-patch", "--stdout", "-1", "HEAD"])
            .output()
            .expect("failed to execute process");

        let mut patch_file = NamedTempFile::new()
            .context(InvalidIoOperationsSnafu)
            .expect("couldn't make temp file");
        patch_file
            .write_all(&output.stdout)
            .expect("couldn't write output to tempfile");
        println!("{:?}", patch_file.path());
        let patch = binding::get_patch_data(patch_file.path().to_path_buf(), path)
            .expect("couldn't get patch");
        for each in patch {
            println!("{:?}", each);
        }
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

    use syn::Expr;
    use syn::LocalInit;
    use syn::Pat;
    use syn::Stmt;
/// Tests advanced parsing and matching capabilities of Rust code, specifically focusing on function bodies and expressions.
/// It reads a Rust file (`../../crates/patchdog/src/binding.rs`), parses all its items,
/// filters for functions, and then for each function, it attempts to parse its body using `syn`.
/// The test includes extensive pattern matching to explore various `Stmt` and `Expr` types within the AST, printing specific identifiers or debug information.
/// This test primarily serves as a diagnostic tool for AST traversal and does not contain explicit assertions for correctness beyond `unwrap()`.
///
/// # Panics
///
/// This test will panic if:
/// - The specified file cannot be read.
/// - `parse_all_rust_items` fails to parse the file content.
/// - `syn::parse_file` fails for any function body.
/// - Any `unwrap()` call fails during AST traversal.
    #[test]
    fn test_parsing_matching() {
        let file = fs::read_to_string("../../crates/patchdog/src/binding.rs").unwrap();
        let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
        let all_fns = parsed
            .iter()
            .filter(|each| each.object_type() == "fn").collect::<Vec<_>>();
        for each in all_fns.to_owned() {
        let function = &FileExtractor::string_to_vector(&file)[each.line_ranges.start..each.line_ranges.end];
        let tokens = syn::parse_file(&function.join("\n")).unwrap();
        let a: &Item = &tokens.items[0];
        match a {
            Item::Fn(item_fn) => {
                //println!("{:#?}", item_fn);
                let a = &item_fn.block.stmts;
                for each in a {
                    match each {
                        Stmt::Expr(expr, _) => {
                            //println!("{:?}", expr);
                            match expr {
                                Expr::Path(path) => {
                                    //println!("{:#?}", path);
                                }
                                Expr::ForLoop(for_loop) => {
                                    //println!("{:#?}", for_loop);
                                    let a = for_loop.body.clone().stmts;
                                    //println!("{:#?}", a);
                                    for each in a {
                                        match each {
                                            Stmt::Expr(expr, _) => {
                                                //println!("{:?}", expr);
                                                match expr {
                                                    Expr::Path(path) => {
                                                        //println!("{:#?}", path);
                                                    }
                                                    _ => {
                                                        ();
                                                    }
                                                }
                                            }
                                            Stmt::Local(local) => {
                                                //println!("{:#?}", local);
                                                let a = local.clone().init.unwrap();
                                                match a {
                                                    LocalInit { expr, .. } => {
                                                        //println!("{:?}", expr);
                                                        match *expr {
                                                            Expr::Path(path) => {
                                                                //println!("{:#?}", path);
                                                            }
                                                            Expr::MethodCall(mc) => {
                                                                //println!("{:#?}", mc)
                                                            }
                                                            Expr::Try(t) => {
                                                                //println!("{:#?}", t);
                                                                match *t.expr {
                                                                    Expr::Call(c) => {
                                                                        let a = c.func.clone();
                                                                        match *a {
                                                                            Expr::Path(path) => {
                                                                                let a = path
                                                                                    .path
                                                                                    .segments;
                                                                                for each in a {
                                                                                    println!("{:#?}", each.ident.to_string());
                                                                                }
                                                                            }
                                                                            _ => {
                                                                                ();
                                                                            }
                                                                        }
                                                                    }
                                                                    _ => {
                                                                        ();
                                                                    }
                                                                }
                                                            }
                                                            _ => {
                                                                ();
                                                            }
                                                        }
                                                    }

                                                    _ => {
                                                        ();
                                                    }
                                                }
                                            }
                                            _ => {
                                                ();
                                            }
                                        }
                                    }
                                    let a = for_loop.clone().pat;
                                    match *a {
                                        Pat::Ident(ident) => {
                                            //println!("{:#?}", ident);
                                            //println!("{:?}", ident.ident.to_string());
                                        }

                                        _ => {
                                            ();
                                        }
                                    }
                                }
                                _ => {
                                    ();
                                }
                            }
                            //println!("{:?}", expr);
                        }
                        Stmt::Local(local) => {
                            //println!("{:?}", local);
                            let a = local.init.clone();
                            //println!("{:#?}", a);
                            match a {
                                Some(LocalInit { expr, .. }) => {
                                    //println!("{:#?}", expr);
                                    match *expr {
                                        Expr::Try(t) => {
                                            let a = t.clone().expr;
                                            match *a {
                                                Expr::Call(call) => {
                                                    let a = call.func;
                                                    match *a {
                                                        Expr::Path(path) => {
                                                            //println!("{:#?}", path.path.get_ident().unwrap().to_string());
                                                        }
                                                        _ => {
                                                            ();
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    ();
                                                }
                                            }
                                            //println!("{:#?}", t.expr);
                                        }
                                        _ => {
                                            ();
                                        }
                                    }
                                    //println!("{:?}", ident.ident.to_string());
                                }
                                _ => {
                                    ();
                                }
                            }
                        }
                        Stmt::Item(item) => {

                            //println!("{:?}", item);
                        }
                        Stmt::Macro(mac) => {
                            //println!("{:?}", mac);
                        }
                        _ => {
                            ();
                        }
                    }
                }
            }
            _ => {
                ();
            }
        }
    }
}
}
