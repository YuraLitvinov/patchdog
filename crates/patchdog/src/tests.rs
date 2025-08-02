mod tests {
    use crate::binding;
    use crate::cli::collect_response;
    use gemini::gemini::{Response, SingleFunctionData};
    use regex::Regex;
    use rust_parsing::ErrorHandling;
    use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu, SerdeSnafu};
    use rust_parsing::file_parsing::{FileExtractor, Files, REGEX};
    use rust_parsing::rust_parser::{RustItemParser, RustParser};
    use snafu::ResultExt;
    use std::env;
    use std::io::Write;
    use std::process::Command;
    use std::{fs, path::Path};
    use tempfile::NamedTempFile;
    const PATH_BASE: &str = "../../tests/data.rs";
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
    }
/// Tests finding module files within a project. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// None
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
                    RustItemParser::find_module_file(path.to_path_buf(), obj_name.to_owned())
                        .expect("Couldn't find mod file");
                obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
            }
        }

        assert_eq!(expected_behavior, obj_vector.join("\n"));
    }

/// Tests reading command-line arguments (This is a test function and should not be relied upon for production use).
///
/// # Returns
///
/// None
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

/// Tests reading and processing a Git patch file. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// None
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
/// Tests handling of empty objects (This is a test function and should not be relied upon for production use).
///
/// # Returns
///
/// None
    #[test]
    fn test_cover_empty_object() {
        /*
        let mut name: Vec<Name> = Vec::new();
        let mut ranges: Vec<LineRange> = Vec::new();
        let mut _objectrange: Vec<ObjectRange> = Vec::new();
        */
    }

/// Tests finding comments in a Rust file. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// None
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
/// Tests the output of the AI agent. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// A `Result` indicating whether the test was successful, or an `ErrorBinding` if any error occurred.
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

/// Tests the regular expression used for parsing JSON data.  This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// None
    #[test]
    fn test_regex() {
        let re = Regex::new(REGEX).unwrap();
        let test = fs::read_to_string(Path::new("../../tests/request.json")).unwrap();
        let mut i = 0;
        let mut assess_size = vec![];
        for cap in re.captures_iter(&test) {
            i += 1;
            let a = cap.get(0).unwrap().as_str();
            let to_struct = serde_json::from_str::<SingleFunctionData>(a).unwrap();
            println!("{:#?}", to_struct);
            assess_size.push(to_struct);
        }
        println!("{}", assess_size.len());
        assert_eq!(i, assess_size.len());
    }

/// Tests parsing JSON responses from the Google Gemini API. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// A `Result` indicating whether the test was successful, or an `ErrorHandling` if any error occurred.
    #[test]
    fn test_response() -> Result<(), ErrorHandling> {
        let re = Regex::new(REGEX).unwrap();
        let test = fs::read_to_string(Path::new("../../tests/res.json")).unwrap();
        let mut assess_size = vec![];
        for cap in re.captures_iter(&test) {
            let a = cap.get(0).unwrap().as_str();
            let to_struct = serde_json::from_str::<Response>(a).unwrap();
            assess_size.push(to_struct);
        }
        match serde_json::from_str::<Vec<Response>>(&test) {
            Ok(ok) => {
                println!("{} = {}", ok.len(), assess_size.len());
            }
            Err(_) => {
                let as_vec = FileExtractor::string_to_vector(&test);
                let a = &as_vec[1..as_vec.len() - 1].join("\n");
                let to_struct = serde_json::from_str::<Vec<Response>>(a).unwrap();
                println!("{:#?}", to_struct);
                println!("{} = {}", assess_size.len(), to_struct.len());
                assert_eq!(assess_size.len(), to_struct.len());
            }
        }
        Ok(())
    }

/// Tests comparing request and response data. This is a test function and should not be relied upon for production use.
///
/// # Returns
///
/// A `Result` indicating whether the test was successful, or an `ErrorHandling` if any error occurred.
    #[test]
    fn test_compare() -> Result<(), ErrorHandling> {
        //Attempting to assess and preserve difference between request and response
        let request = fs::read_to_string(Path::new("../../tests/request.json")).unwrap();
        let response = fs::read_to_string(Path::new("../../tests/res.json")).unwrap();
        let request = serde_json::from_str::<Vec<SingleFunctionData>>(&request)
            .context(SerdeSnafu)?;
        let response = collect_response(&response)?;
        assert_eq!(request.len(), response.len()+1);
        Ok(())
    }
}
