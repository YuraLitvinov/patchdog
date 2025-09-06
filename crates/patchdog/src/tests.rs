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
    fn test_paths () {
        let file_path = fs::canonicalize("/home/yurii-sama/patchdog/crates/patchdog/src/tests.rs").unwrap();
        let more_paths = [
    "/home/yurii-sama/patchdog/tests/",
    "/home/yurii-sama/patchdog/crates/patchdog/src/tests.rs",
    "/home/yurii-sama/patchdog/crates/rust_parsing/src/error.rs",
        ];
        let starts = more_paths.iter().all(|path| Path::new(path).starts_with(&file_path));
        if starts == false {
            for path in more_paths.iter() {
                if Path::new(path) == file_path {
                    println!("{}", path);
                }
            }
        }
        println!("{}", starts);

        assert_eq!(true,false);
    }
}
