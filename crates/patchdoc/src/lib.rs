use std::collections::hash_set::Difference;
use std::path::Path;

use filesystem_parsing::{file_deserialize, file_to_vector};
use gemini::GoogleGemini;
use filesystem_parsing::parse;
use filesystem_parsing::write_to_file;

pub async fn finalized(project_file:&'static str) {
    match file_deserialize(project_file) {
        Ok(paths) => for path in paths {
            println!("{}", &path);
            let read_file = |path: &str| std::fs::read_to_string(path).expect("No such file");
            let contents = read_file(&path);
            let for_parse = contents.clone();
            let parsed = parse(for_parse).join(" ");
            println!("{parsed:?}");
            let to_agent = parsed + " - Use this as reference for the objects that have to be documented\n " + &contents;
            let test1 = GoogleGemini::req_res(to_agent).await;

            let output = match test1 {
                Ok(res) => res,
                Err(why) => why.to_string(),
            };


            let _ = write_to_file(output, path);
        },
        Err(why) => { eprintln!("{}", why); }
    };
}
//finalized("project_files.json").await;
    //let input: String = my_parse_file("src/lib.rs");
    //write_to_file(input, "Tree_of_file".to_string());
    //println!("{}", input);
    //let transform = transform(input);
    //let output =  match GoogleGemini::req_res(input).await {
    //    Ok(res) => write_to_file(res, "zhopa".to_string()),
    //    Err(why) => write_to_file(why.to_string(),  "zhopa".to_string()),
    //};
    //let _ = output;
    //println!("{}", output);
 
//Compares two file paths for any differences between them. 
//If it notices some diff, then it will return the diff and parse all objects that are within the line range
/* 
pub fn compare(file_to_compare: &Path, file_comparable: &Path) {
    let file_to_compare = file_to_vector(file_to_compare).join("\n");
    let file_comparable = file_to_vector(file_comparable).join("\n");
    let diff = file_to_compare.diff(&file_comparable);
    println!("{}", diff);

   

}
*/
