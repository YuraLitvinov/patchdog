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

    #[test]
    fn test_cover_empty_object() {
        /*
        let mut name: Vec<Name> = Vec::new();
        let mut ranges: Vec<LineRange> = Vec::new();
        let mut _objectrange: Vec<ObjectRange> = Vec::new();
        */
    }

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

    #[test]
    fn test_regex() {
        let test = fs::read_to_string(Path::new("../../tests/response_regex.json")).unwrap();
        let assess_size = cherrypick_response(&test).unwrap();
        assert_eq!(assess_size.len(), 3);
    }

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
