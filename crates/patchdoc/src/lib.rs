//use std::collections::hash_set::Difference;
use std::path::Path;
use filesystem_parsing::extract_function;
use filesystem_parsing::{file_deserialize, parse_all_rust_items, frontend_visit_items, find_module_file};
use gemini::GoogleGemini;
use filesystem_parsing::parse;
use filesystem_parsing::write_to_file;
use filesystem_parsing::file_to_vector;
use similar::{ChangeTag, TextDiff};
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
pub fn receive_context(file_path: &Path) -> Vec<String> { 
let visited = parse_all_rust_items(file_path);
let mut vector_of_objects: Vec<String> = Vec::new();
println!("{:?}", &file_path);
    for item in &visited {
    println!("{:?}", frontend_visit_items(item));
    vector_of_objects.push(item.object_name().unwrap());
        if item.object_type().unwrap() == "mod" {
            //println!("{:?}", &file_path.parent().unwrap());
            let mod_path = find_module_file(file_path.parent().unwrap(), &item.object_name().unwrap());
            let path2 = &mod_path.clone().unwrap();
            let parsed = parse_all_rust_items(&mod_path.unwrap());
            for item in &parsed {
                //println!("\t{:?}", frontend_visit_items(&item));
                println!("\t{:?}", &item);
                let extr = extract_function(path2, &item.line_start().unwrap(), &item.line_end().unwrap());
                vector_of_objects.push(extr.clone());
                //println!("\t{}", extr);
            }
        }
    }
    vector_of_objects
}



pub fn compare(file_to_compare: &Path, file_comparable: &Path) {
    let file_to_compare = file_to_vector(file_to_compare).join("\n");
    let file_comparable = file_to_vector(file_comparable).join("\n");
    let diff = TextDiff::from_lines(&file_comparable, &file_to_compare);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        println!("{} {}", sign, change);
    }
}

