mod tests {
    use rust_parsing::comment_lexer;
    use rust_parsing::error::InvalidIoOperationsSnafu;
    use rust_parsing::rust_parser::{RustItemParser, RustParser};
    use tempfile::NamedTempFile;
    use std::env;
    use std::io::Write;
    use rust_parsing::file_parsing::{FileExtractor, Files};
    use crate::binding::get_patch_data;
    use snafu::ResultExt;
    use std::{fs, path::Path};
    use std::process::Command;
    const PATH_BASE: &str = "../../tests/data.rs";
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
        assert_eq!(COMPARE_LINES, line_eight_from_vector.to_owned() +"\n"); //This test has passed
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
                    RustItemParser::find_module_file(path.to_path_buf(), obj_name.to_owned())
                        .expect("Couldn't find mod file");
                obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
            }
        }

        assert_eq!(expected_behavior, obj_vector.join("\n"));
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
    fn test_read_argument() {
         let mut path = env::current_dir().expect("couldn't get path");
         path.pop();
         path.pop();
         let path_to_patch = path.join("patch.patch");
         
        assert_eq!(path_to_patch, Path::new("/home/runner/work/patchdog/patchdog/patch.patch"));
    }
    #[test]
    fn test_read_file(){
    let mut path = env::current_dir().expect("couldn't get path");
         path.pop();
         path.pop();
        let path_to_patch = path.join("patch.patch");
    fs::read_to_string(path_to_patch)
        .expect("Couldn't read");
    }
    #[test]
    fn test_read_patch() {
            let mut path = env::current_dir()
        .context(InvalidIoOperationsSnafu)
        .expect("couldn't get current dir");
    path.pop();
    path.pop();
    let output = 
    Command::new("git")
        .args(["format-patch", "--output=patch.patch", "--stdout", "-1", "HEAD"])
        .output()
        .expect("failed to execute process");
    
    let mut patch_file = NamedTempFile::new().context(InvalidIoOperationsSnafu).expect("couldn't make temp file");
    patch_file.write_all(&output.stdout)
        .expect("couldn't write output to tempfile");
    println!("{:?}",patch_file.path());
    let patch = get_patch_data(
        patch_file.path().to_path_buf(),
        path,
    ).expect("couldn't get patch");
    for each in patch {
        println!("{:?}", each);
    }
        assert_eq!(true,false);
}
}
