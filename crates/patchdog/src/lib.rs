use filesystem_parsing::parse_all_rust_items;
use filesystem_parsing::{ObjectRange, extract_function};
use std::path::Path;
/*
pub fn receive_context(file_path: &Path) -> Vec<String> {
    let visited = match parse_all_rust_items(file_path) {
        Ok(visited) => visited,
        Err(why) => {
            eprintln!("{}", why);
            return Vec::new();
        }
    };
    let mut vector_of_objects: Vec<String> = Vec::new();
    for item in &visited {
        vector_of_objects.push(item.object_name().unwrap());
        if item.object_type().unwrap() == "mod" {
            let mod_path =
                match find_module_file(file_path.parent().unwrap(), &item.object_name().unwrap()) {
                    Ok(path) => path,
                    Err(why) => {
                        eprintln!("{}", why);
                        None
                    }
                };
            let path2 = &mod_path.unwrap();
            let parsed = match parse_all_rust_items(path2) {
                Ok(parsed) => parsed,
                Err(why) => {
                    eprintln!("{}", why);
                    return Vec::new();
                }
            };
            for item in &parsed {
                let extr = extract_function(
                    path2,
                    &item.line_start().unwrap(),
                    &item.line_end().unwrap(),
                );
                vector_of_objects.push(extr.unwrap());
            }
        }
    }
    vector_of_objects
}
*/

pub fn seeker(line_index: usize, item: ObjectRange, from: &Path) -> String {
    let line_start = item.line_start().unwrap();
    let line_end = item.line_end().unwrap();
    if line_start > line_end {
        let err = "Line start can't be greater than line end".to_string();
        return err;
    }
    if line_start <= line_index && line_end >= line_index {
        let extracted = match extract_function(from, &line_start, &line_end) {
            Ok(extracted) => extracted,
            Err(why) => {
                let err = format!("Failed to extract function: {}", why);
                return err;
            }
        };
        return extracted;
    }
    let err = format!("Line index {} is out of bounds", line_index);
    err
}

pub fn receive_context(line_from: usize, file_path: &Path) -> String {
    let visited = match parse_all_rust_items(file_path) {
        Ok(visited) => visited,
        Err(why) => {
            return why.to_string();
        }
    };
    for item in visited {
        let found = seeker(line_from, item, file_path);
        if found == *"Line index out of bounds" {
            continue;
        }
        return found;
    }
    let err = format!("Line index {} is out of bounds", line_from);
    err
}
