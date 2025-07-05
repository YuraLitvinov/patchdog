#[cfg(test)]
mod tests {

    const PATH: &'static str = "/home/runner/work/patchdog/patchdog/tests/data.rs";
    use anyhow::Context;
    use rust_parsing::{InvalidIoOperationsSnafu, InvalidSynParsingSnafu};
    use rust_parsing::{extract_by_line, file_to_vector, parse_all_rust_items, string_to_vector};
    use std::fs;
    use std::path::Path;

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
        let function_from_file =
            fs::read_to_string("/home/yurii-sama/Desktop/patchdog/crates/gemini/src/lib.rs")
                .context(format!("{:?}", InvalidIoOperationsSnafu))
                .unwrap();
        let vector_of_file = string_to_vector(function_from_file);
        let extracted_object = extract_by_line(vector_of_file, &start, &end)
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let object_from_const = format!("{}", IMPL_GEMINI);
        assert_ne!(object_from_const, extracted_object);
    }

    #[test]
    fn test_string_vs_file() {
        //Actually an impl block; doesn't affect the result
        let function_from_file =
            fs::read_to_string("/home/yurii-sama/Desktop/patchdog/crates/gemini/src/lib.rs")
                .context(format!("{:?}", InvalidIoOperationsSnafu))
                .unwrap();
        let vector_from_string = string_to_vector(function_from_file);
        let vector_from_file = file_to_vector(Path::new(
            "/home/yurii-sama/Desktop/patchdog/crates/gemini/src/lib.rs",
        ))
        .context(format!("{:?}", InvalidIoOperationsSnafu))
        .unwrap();
        assert_eq!(vector_from_file, vector_from_string);
    }

    const COMPARE_LINES: &str = "pub struct GoogleGemini; //Req Res = Request Response"; //this outputs exists at line 8 in the code at the moment of testing
    #[test]
    fn test_file_to_vector() {
        //file_to_vectors splits a file into a string of vectors line by line
        let vectored_file = file_to_vector(Path::new(
            "/home/yurii-sama/Desktop/patchdog/crates/gemini/src/lib.rs",
        ))
        .context(format!("{:?}", InvalidIoOperationsSnafu))
        .unwrap();
        let line_eight_from_vector = &vectored_file[7]; //Count in vec! starts from 0 
        assert_eq!(COMPARE_LINES, line_eight_from_vector); //This test has passed
    }
    #[test]
    fn test_parse() {
        let source = fs::read_to_string(Path::new(PATH))
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let parsed = parse_all_rust_items(&source)
            .context(format!("{:?}", InvalidSynParsingSnafu))
            .unwrap();
        for object in parsed {
            let obj_type = object.object_type().unwrap();
            if obj_type == "impl".to_string() {
                println!("{:?}", object);
            }
        }

        assert_ne!(true, true);
    }
    #[test]
    fn find_all_fn() {
        let source = fs::read_to_string(Path::new(PATH))
            .context(format!("{:?}", InvalidIoOperationsSnafu))
            .unwrap();
        let parsed = parse_all_rust_items(&source).unwrap();
        for object in parsed {
            let obj_type = object.object_type().unwrap();
            if obj_type == "fn".to_string() {
                println!("{:?}", object);
            }
        }
        assert_ne!(true, true);
    }
}

/*
    #[test]
    fn testing_seeker_for_use() {
        let string_of_func = "use std::collections::{HashMap,
    HashSet,
    VecDeque};";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(2, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }

    #[test]
    fn testing_seeker_for_zero() {
        let string_of_func: &'static str = "LineOutOfBounds";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(0, path);
        let formatted_receive = receive.unwrap_err().to_string();
        assert!(formatted_receive.contains(string_of_func));
    }
    #[test]
    fn testing_seeker_for_out_of_bounds() {
        let string_of_func: &'static str = "LineOutOfBounds";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(999999, path);
        let formatted_receive = receive.unwrap_err().to_string();
        assert!(formatted_receive.contains(string_of_func));
    }
    #[test]
    fn find_impl() {
        let string_of_func = r#"impl Item {
    fn new(
        name: &str,
        item_type: ItemType,
        price: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            item_type,
            price,
            status: Status::Active,
        }
    }

    fn deactivate(
        &mut self
    ) {
        self.status = Status::Inactive;
    }
}"#;
        let path = Path::new(TEST_PATH);
        let receive = receive_context(43, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }

    #[test]
    fn find_function() {
        let string_of_func = r#"fn bookshop(
    name: &str,
    item_type: ItemType,
    price: f32,
) -> Self {
    Self {
        name: name.to_string(),
        item_type,
        price,
        status: Status::Active,
    }
}"#;
        let path = Path::new(TEST_PATH);
        let receive = receive_context(166, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }


    //const PATCH: &'static str = r#""#;



    #[test]
    fn test_from_buffer() {

        let mut write = fs::read("/home/yurii-sama/Desktop/patchdog/write.patch").expect("Reading failed");

        let diff_from_patch = Diff::from_buffer(write.as_mut_slice())
            .expect("Diff error");
        for (i, _diff) in diff_from_patch.deltas().enumerate() {
        let patch = Patch::from_diff(&diff_from_patch, i).unwrap();
        println!("{:?}", patch);
            //eprintln!("{:?}", diff);
            //git2::Patch::line_stats(diff)



        }
        assert_eq!(false, true);
    }
*/
/*
    #[test]
    fn seeker_on_functions() {
    let funcs_from_file = fs::read_to_string("/home/yurii-sama/Desktop/patchdog/tests/data.rs")
        .context(format!("{:?}",InvalidIoOperationsSnafu)).unwrap();
    let receive = receive_context(22, funcs_from_file).unwrap();
    println!("{}", receive);
    assert_eq!(receive, "");

}
*/
