mod tests {
    use crate::cli::cherrypick_response;
    use gemini::request_preparation::RawResponse;
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::file_parsing::{Files, FsOperations, REGEX};
    use rust_parsing::object_range::Name;
    use rust_parsing::{
        ObjectRange,
        rust_parser::{RustItemParser, RustParser},
    };
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
                let as_vec = FsOperations::string_to_vector(&test);
                let a = &as_vec[1..as_vec.len() - 1].join("\n");
                let to_struct = serde_json::from_str::<Vec<RawResponse>>(a).unwrap();
                assert_eq!(assess_size.len(), 3);
                assert_eq!(to_struct.len(), 58);
            }
        }
        Ok(())
    }

    #[test]
    fn test_path_tests() {
        let mut dir = env::current_dir().unwrap();
        dir.pop();
        dir.pop();
        let file_path_test = dir.join("tests/data.rs").display().to_string();
        let more_paths = ["tests/*", "crates/**/src/tests.rs", "crates/**/src/main.rs"]
            .iter()
            .map(|s| dir.join(s).display().to_string())
            .collect::<Vec<String>>();
        let val =
            crate::binding::is_file_allowed(&Path::new(&file_path_test), &more_paths).unwrap();

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
        let more_paths = ["tests/", "crates/**/src/tests.rs", "crates/**/src/main.rs"]
            .iter()
            .map(|s| dir.join(s).display().to_string())
            .collect::<Vec<String>>();
        let val =
            crate::binding::is_file_allowed(&Path::new(&file_path_main), &more_paths).unwrap();

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
        let parsed = RustItemParser::parse_all_rust_items(&file)
            .expect("Failed to parse")
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
            },
        ];
        assert!(parsed == objects);
    }

    #[test]
    fn test_lexer() {
        let mut dir = env::current_dir().expect("Failed to get current directory");
        dir.pop();
        dir.pop();
        let filepath = dir.join("tests/data.rs").display().to_string();
        let file = fs::read_to_string(filepath).expect("Failed to read file");
        let parsed = rust_parsing::comment_lexer(&file)
            .expect("Failed to parse")
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
}
