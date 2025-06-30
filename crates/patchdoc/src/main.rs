use dotenv::dotenv;
use filesystem_parsing::parse_all_rust_items;
use std::path::Path;
//use std::env;
//use filesystem_parsing::frontend_visit_items;
use filesystem_parsing::file_to_vector;
use similar::{ChangeTag, TextDiff};

#[tokio::main]


async fn main() {
    dotenv().ok();    
   // let args: Vec<String> = env::args().collect();
    //let file_path = Path::new(&args[1]);
   //let line_start = &args[2].parse::<usize>().expect("Not a number");
   // let line_end = &args[3].parse::<usize>().expect("Not a number");
    //frontend_visit_items(file_path, line_start, line_end);
   let test = parse_all_rust_items(&Path::new("crates/filesystem_parsing/src/lib.rs"));
   println!("{:?}", test);
    
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
