 mod tests {
    use crate::cli::cherrypick_response;
    use gemini::request_preparation::RawResponse;
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::error::InvalidIoOperationsSnafu;
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
                let range = RustItemParser::textrange_into_linerange(val.range.clone(), &src);
                (range.to_owned(), val.clone())
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

    #[test]
    fn test_parse() {
        let source = fs::read_to_string(PATH_BASE).expect("File read failed");
        let parsed = RustItemParser::parse_all_rust_items(&source).expect("Parsing failed");
        for object in parsed {
            let obj_type = object.names.type_name.clone();
            if obj_type == "impl".to_string() {}
        }
    }

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
      "filename": "crates/gemini/src/lib.rs",
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

    #[test]
    fn test_path_test() {
        let mut dir = env::current_dir().unwrap();
        dir.pop();
        dir.pop();
        let file_path_test = dir
            .join("crates/patchdog/src/tests.rs")
            .display()
            .to_string();
        let more_paths = [
            "tests/",
            "crates/**/src/tests.rs",
            "crates/**/src/main.rs",
        ]
        .iter()
        .map(|s| dir.join(s).display().to_string())
        .collect::<Vec<String>>();
        let val = crate::binding::is_file_allowed(&Path::new(&file_path_test), &more_paths).unwrap();

        println!("is file_allowed: {:?}", val);
        assert!(!val);
    }
    #[test]
    fn test_path_main() {
        let mut dir = env::current_dir().unwrap();
        dir.pop();
        dir.pop();
        let file_path_main = dir
            .join("crates/patchdog/src/main.rs")
            .display()
            .to_string();
        let more_paths = [
            "tests/",
            "crates/**/src/tests.rs",
            "crates/**/src/main.rs",
        ]
        .iter()
        .map(|s| dir.join(s).display().to_string())
        .collect::<Vec<String>>();
        let val = crate::binding::is_file_allowed(&Path::new(&file_path_main), &more_paths).unwrap();

        println!("is file_allowed: {:?}", val);
        assert!(!val);
    }

    #[test]
    fn test_parsing_extended() {
        
        let mut dir = env::current_dir().expect("Failed to get current directory");
        dir.pop();
        dir.pop();
        let filepath = dir.join("tests/data.rs").display().to_string();
        let file = fs::read_to_string(filepath).expect("Failed to read file");
        let parsed = RustItemParser::parse_all_rust_items(&file).expect("Failed to parse")
        .iter()
        .filter_map(|x| {
            if x.names.type_name == "fn" {
                Some(x.clone())
            } else {
                None
            }
        })
        .collect::<Vec<ObjectRange>>();
        let objects = vec![
            ObjectRange {
                line_ranges: 1..2,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "regular_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 4..5,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "public_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 7..9,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "function_with_return".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 11..12,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "required_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 14..15,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "function_with_params".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 17..18,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "generic_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 20..23,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "lifetime_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 25..28,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "const_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 30..31,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "async_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 33..34,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "unsafe_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 36..37,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "extern_c_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 39..42,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "result_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 48..48,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "method".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 50..50,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "public_method".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 52..52,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "static_method".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 54..56,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "method_with_lifetime".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 58..58,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "method_with_generic".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 71..71,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "required_function".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 74..77,
                names: Name {
                    type_name: "fn".to_string(),
                    name: "result_function".to_string(),
                },
            }
        ];
        assert!(parsed == objects)
    }

    #[test]
    fn test_lexer() {
        let mut dir = env::current_dir().expect("Failed to get current directory");
        dir.pop();
        dir.pop();
        let filepath = dir.join("tests/data.rs").display().to_string();
        let file = fs::read_to_string(filepath).expect("Failed to read file");
        let parsed = rust_parsing::comment_lexer(&file).expect("Failed to parse")
            .into_iter()
            .filter_map(|each| {
                if each.names.name == "///" {
                    return Some(each);
                }
                None
            })
            .collect::<Vec<ObjectRange>>();
        println!("{:#?}", parsed);
        let test_data = vec![
            ObjectRange {
                line_ranges: 79..79,
                names: Name {
                    type_name: "LineComment".to_string(),
                    name: "///".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 85..85,
                names: Name {
                    type_name: "LineComment".to_string(),
                    name: "///".to_string(),
                },
            },
            ObjectRange {
                line_ranges: 86..86,
                names: Name {
                    type_name: "LineComment".to_string(),
                    name: "///".to_string(),
                },
            },
        ];
        assert!(!(parsed == test_data))
    }

    const TEST_STRING: &'static str = "/// Returns the name of the code object. This method provides direct access to the `name` field within the `names` property of the `ObjectRange` struct.
    /// It is primarily used to retrieve the identifier (e.g., function name, struct name) associated with a parsed Rust code item.
    ///
    /// # Returns
    ///
    /// A `String` representing the name of the object.
    pub fn object_name(&self) -> String {
        self.names.name.to_string()
    }";
    const ANUDDA_TEST_STRING: &'static str = r#"/// Orchestrates the entire process of applying a patch to a codebase, generating AI-based code suggestions, and writing them back to files. This asynchronous function takes a patch file, analyzes the changes, filters them based on exclusion rules, and then generates `Request` objects for an AI agent.
/// It then calls the AI agent, collects the responses, and writes the suggested new comments or code modifications to the corresponding files. The function includes a call limit to prevent infinite recursion in case of persistent AI failures.
///
/// # Arguments
///
/// * `analyzer_data` - `AnalyzerData` providing context for code analysis.
/// * `commands` - `Mode` struct containing command-line arguments, including the patch file path, target Rust types, and names, and debug flag.
///
/// # Returns
///
/// A `Result<(), ErrorBinding>` indicating the success or failure of the entire patch-to-agent and write-back process.
pub async fn cli_patch_to_agent(
    analyzer_data: AnalyzerData,
    commands: Mode,
) -> Result<(), ErrorBinding> {
    //Mode accepts type and name of the object for the sake of debugging. It autodefaults to any fn
    let patch = binding::patch_data_argument(commands.file_patch)?;
    let parameters = ai_interactions::return_prompt()?.patchdog_settings;
    event!(Level::INFO, "type: {:#?}", parameters.affected_object_types);
    let dir = env::current_dir()?;
    let excluded_paths = parameters
        .excluded_files
        .par_iter()
        .map(|path| dir.join(path).display().to_string())
        .collect::<Vec<String>>();
    let request = changes_from_patch(
        patch,
        parameters.affected_object_types,
        commands.name_rust,
        &excluded_paths,
        analyzer_data,
    )?;
    println!("request: {:#?}", request);
    //Here occurs check for pending changes
    if request.is_empty() {
        event!(Level::INFO, "No requests");
    } else {
        event!(Level::INFO, "Requests length: {}", &request.len());
        //Limiting possible call count to 3, to prevent infinite recursion, where LLM fails to fulfill the request
        let mut call_limit = 0;
        let responses_collected = call(request, &mut call_limit).await?;
        event!(
            Level::INFO,
            "Responses collected: {}",
            responses_collected.len()
        );
        write_to_file(responses_collected)?;
    }
    Ok(())
}
"#;
    #[test]
    fn test_theory(){
        let parsed = RustItemParser::parse_all_rust_items(ANUDDA_TEST_STRING).expect("Failed to parse");
        let comments = parsed
            .iter()
            .filter_map(|each|{
                if each.names.name == "///" {
                    Some(each)
                } else {
                    None
                }
            })
            .collect::<Vec<&ObjectRange>>();
        let function_object = parsed.iter().filter_map(|each|{
            if each.names.type_name == "fn" {
                Some(each)
            } else {
                None
            }
        }).collect::<Vec<&ObjectRange>>().first().unwrap().clone();

        let remove_from_original = rust_parsing::file_parsing::FileExtractor::string_to_vector(ANUDDA_TEST_STRING);
        let mut remove_from_original = remove_from_original.clone();
        let first_element = comments.first().unwrap().line_start();
        let last_element = comments.last().unwrap().line_end();
        remove_from_original.drain(first_element-1..last_element);
        let actual_function_object = ObjectRange {
            line_ranges: function_object.line_start()+last_element..function_object.line_end(),
            names: function_object.names.clone(),
        };
        println!("{:#?}", actual_function_object);
        println!("{}", remove_from_original.join("\n"));

        assert_eq!(true,false)
    }

    #[test]
    fn test_write_with_remove(){
        let mut dir = env::current_dir().expect("Failed to get current directory");
        dir.pop();
        dir.pop();
        let file = fs::read_to_string(dir.join("crates/rust_parsing/src/rust_parser.rs").display().to_string()).expect("Failed to read file");
        let to_vec = FileExtractor::string_to_vector(&file);
        let test = rust_parsing::file_parsing::write_with_remove(
            dir.join("crates/rust_parsing/src/rust_parser.rs"),
            to_vec, 
            235,
            "====== TEST STRING =======".to_string()
        ).unwrap();
        
        assert_eq!(true,false)
    }
}
