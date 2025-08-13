mod tests {
    use crate::binding;
    use crate::cli::cherrypick_response;
    use gemini::gemini::{RawResponse};
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu};
    use rust_parsing::file_parsing::{FileExtractor, Files, REGEX};
    use rust_parsing::rust_parser::{RustItemParser, RustParser};
    use snafu::ResultExt;
    use std::env;
    use std::io::Write;
    use std::process::{Command};
    use std::{fs, path::Path};
    use tempfile::NamedTempFile;
    use syn::Expr;
    use syn::Pat;
    use syn::Item;
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
        let function = &FileExtractor::string_to_vector(&file)[35..=102];
        let tokens = syn::parse_file(&function.join("\n")).unwrap().items;
        for each in tokens {
            entry_point(each);
        }
        
    assert_eq!(true,false);
    }
    fn entry_point(token: Item) {
        match token {
            Item::Fn(f) => read_block(*f.block),
            _ => ()
            
        }
    }
fn match_expr(expr: Expr) {
    match expr {
        Expr::Assign(assign) => {
            match_expr(*assign.left);
            match_expr(*assign.right);
        },
        Expr::Block(block) => read_block(block.block),
        Expr::Call(call) => {
            match_expr(*call.func);
            for arg in call.args {
                match_expr(arg);
            }
        }
        Expr::Closure(closure) => match_expr(*closure.body),
        Expr::ForLoop(for_loop) => {
            handle_pat(*for_loop.pat);
            match_expr(*for_loop.expr);
            read_block(for_loop.body);
        }
        Expr::If(if_expr) => {
            match_expr(*if_expr.cond);
            read_block(if_expr.then_branch);
            if let Some((_, else_expr)) = if_expr.else_branch {
                match_expr(*else_expr);
            }
        }
        Expr::Let(let_expr) => {
            match_expr(*let_expr.expr);
            handle_pat(*let_expr.pat);
        }
        Expr::Loop(loop_expr) => read_block(loop_expr.body),
        Expr::Match(m_expr) => {
            match_expr(*m_expr.expr);
            for arm in m_expr.arms {
                handle_pat(arm.pat);
                if let Some((_, guard)) = arm.guard {
                    match_expr(*guard);
                }
                match_expr(*arm.body);
            }
        }
        Expr::MethodCall(method_call) => handle_method_call(method_call),
        Expr::Struct(strukt) => handle_struct(strukt),
        Expr::Path(path_expr) => handle_path(path_expr),
        Expr::Try(try_expr) => match_expr(*try_expr.expr),
        Expr::TryBlock(try_block) => read_block(try_block.block),
        Expr::Unsafe(unsafe_expr) => read_block(unsafe_expr.block),
        Expr::While(while_expr) => {
            match_expr(*while_expr.cond);
            read_block(while_expr.body);
        }
        _ => {}
    }
}

// --- helpers ---

fn handle_pat(pat: Pat) {
    match pat {
        //Pat::Ident(i) => println!("{}", i.ident),
        Pat::Struct(ps) => {
            for field in ps.fields {
                handle_pat(*field.pat);
            }
        }
        _ => {}
    }
}

fn handle_method_call(method_call: syn::ExprMethodCall) {
    println!("{}", method_call.method);
    match_expr(*method_call.receiver);
    for arg in method_call.args {
        match_expr(arg);
    }
}

fn handle_struct(strukt: syn::ExprStruct) {
    if let Some(ident) = strukt.path.get_ident() {
        println!("Struct: {}", ident);
    }
}

fn handle_path(path_expr: syn::ExprPath) {
    for segment in &path_expr.path.segments {
        println!("{}", segment.ident);
    }
}


fn read_block(block: syn::Block) {
    for stmt in block.stmts {
        match stmt {
            syn::Stmt::Expr(e,_) => {
                match_expr(e);
            },
            syn::Stmt::Local(local) => {
                handle_pat(local.pat);
                match_expr(*local.init.unwrap().expr);
            }
            _ => {}
        }
    }
}
}

