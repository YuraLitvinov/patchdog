use dotenv::dotenv;
use filesystem_parsing::parse_all_rust_items;
use std::path::Path;
use std::env;
use filesystem_parsing::frontend_visit_items;
use filesystem_parsing::file_to_vector;
use similar::{ChangeTag, TextDiff};
use filesystem_parsing::find_module_file;
#[tokio::main]


async fn main() {
    dotenv().ok();    
    let args: Vec<String> = env::args().collect();
    let file_path = Path::new(&args[1]);
    let visited = parse_all_rust_items(&file_path);
    for item in &visited {
        frontend_visit_items(&item);
        if item.object_type().unwrap() == "mod" {
            //println!("{:?}", &file_path.parent().unwrap());
            let mod_path = find_module_file(file_path.parent().unwrap(), &item.object_name().unwrap());
            let parsed = parse_all_rust_items(&mod_path.unwrap());
            for item in &parsed {
                frontend_visit_items(&item);
            }
        }
    }
    /* 
    let mod_path = find_module_file(file_path, &visited[0].object_name().unwrap());
    if let Some(mod_file) = mod_path {
        parse_all_rust_items(&mod_file);
    }
    */
}

fn compare(file_to_compare: &Path, file_comparable: &Path) {
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
