
use git_parsing::{match_patch_with_parse, Git2ErrorHandling};
use std::fs;
use rust_parsing::{ObjectRange, object_parsing::parse_all_rust_items};
#[derive(Debug)]
pub struct Surplus {
    pub name: String,
    pub object_type: Vec<ObjectRange>,
}
pub fn store_objects(relative_path: &str, patch_src: &[u8]) -> Result<Vec<Surplus>, Git2ErrorHandling>{ 
    let mut vec_of_surplus: Vec<Surplus> = Vec::new();
     let matched = match_patch_with_parse(relative_path, patch_src).unwrap();
     for change_line in matched {
         if change_line.quantity == 1 {
         let path = relative_path.to_string() + &change_line.change_at_hunk.filename();
         let file = fs::read_to_string(path)
             .expect("Failed to read file");
         let parsed = parse_all_rust_items(file.clone())
         .expect("Failed to parse");
         vec_of_surplus.push(Surplus {
             name: change_line.change_at_hunk.filename(),
             object_type: parsed,
         });
     }            
     }
         
    Ok(vec_of_surplus)
}
