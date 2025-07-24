use rust_parsing::{ErrorHandling, ObjectRange};
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::FunctionSignature;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::{env, fs};
use std::ops::Range;
use std::path::{Path, PathBuf};
use rust_parsing::error::{CouldNotGetLineSnafu, InvalidIoOperationsSnafu, ErrorBinding};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use snafu::{ResultExt, OptionExt};
#[derive(Debug)]
pub struct Export {
    pub filename: PathBuf,
    pub range: Vec<Range<usize>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct FileFn {
    pub filename: String,
    pub types: Vec<FnDataEntry>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub filename: String,
    pub types: Types,
}
//Each of the Types share same information for simplicity
#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub comment: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FnDataEntry {
    //Contains: line range, name, type
    pub generic_information: ObjectRange,
    //Contains: input type, return type
    pub fn_top_block: FunctionSignature,
    //Comment to be filled in by the LLM
    pub comment: String,
}

#[derive(Debug)]
pub struct Types {
    pub fn_: Vec<FnDataEntry>,
    pub impl_: Vec<Entry>,
    pub const_: Vec<Entry>,
    pub struct_: Vec<Entry>,
    pub enum_: Vec<Entry>,
    pub trait_: Vec<Entry>,
    pub type_: Vec<Entry>,
}

// Each entry has name and comment fields

#[derive(Debug, Serialize, Deserialize)]
pub struct Root {
    pub files: Vec<File>,
}

// To handle Rust keywords as field names, use serde renaming:

impl<'de> Deserialize<'de> for Types {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TypesHelper {
            #[serde(rename = "fn")]
            fn_: Vec<FnDataEntry>,
            #[serde(rename = "impl")]
            impl_: Vec<Entry>,
            #[serde(rename = "const")]
            const_: Vec<Entry>,
            #[serde(rename = "struct")]
            struct_: Vec<Entry>,
            #[serde(rename = "enum")]
            enum_: Vec<Entry>,
            #[serde(rename = "trait")]
            trait_: Vec<Entry>,
            #[serde(rename = "type")]
            type_: Vec<Entry>,
        }

        let helper = TypesHelper::deserialize(deserializer)?;
        Ok(Types {
            fn_: helper.fn_,
            impl_: helper.impl_,
            const_: helper.const_,
            struct_: helper.struct_,
            enum_: helper.enum_,
            trait_: helper.trait_,
            type_: helper.type_,
        })
    }
}

impl Serialize for Types {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Types", 7)?;
        state.serialize_field("fn", &self.fn_)?;
        state.serialize_field("impl", &self.impl_)?;
        state.serialize_field("const", &self.const_)?;
        state.serialize_field("struct", &self.struct_)?;
        state.serialize_field("enum", &self.enum_)?;
        state.serialize_field("trait", &self.trait_)?;
        state.serialize_field("type", &self.type_)?;
        state.end()
    }
}

pub fn assess_correct_output(response: String) -> Result<bool, ErrorBinding> {
    let mut stripped = FileExtractor::string_to_vector(&response);
    stripped.remove(0).remove(stripped.len());
    println!("{}", stripped.join(""));
    let parsed: Result<Root, serde_json::Error> = serde_json::from_str(&response);
    match parsed {
        Ok(p) => {
            for each in &p.files {
                let path = Path::new(&each.filename);
                if path.exists() {
                    let mut rust_files: Vec<PathBuf> = Vec::new();
                    let mut fn_names: Vec<String> = Vec::new();
                    rust_files.push(path.to_path_buf());
                    let file_export = make_export(&rust_files)?;
                    println!("valid path: {:?}", &each.filename);
                    let types = &each.types;
                    for funcs in &types.fn_ {
                        fn_names.push(funcs.generic_information.object_name().expect("err"));
                    }
                    let exported =
                        justify_presence(file_export, vec!["fn".to_string()], fn_names.clone())?;

                    if &types.fn_.len() == &exported.len() {
                        println!("PASS: amount of matches: {}", exported.len());
                    } else {
                        let err = format!(
                            "JSON count: {} actual count: {}",
                            &types.fn_.len(),
                            &exported.len()
                        );
                        call_agent(err);
                    }
                } else {
                    let err = format!("Path: {:?} exists: {}", path, path.exists());
                    call_agent(err);
                }
            }
        }
        Err(e) => {
            let err = format!("{:?}", e);
            call_agent(err);
        }
    }
    Ok(true)
}
pub fn call_agent(err: String) {
    println!("called call_agent with outcome\n{err}");
}

fn justify_presence(
    exported_from_file: Vec<Export>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
) -> Result<Vec<bool>, ErrorBinding> {
    let mut vecbool: Vec<bool> = Vec::new();
    for each_item in exported_from_file {
        let file = fs::read_to_string(&each_item.filename).context(InvalidIoOperationsSnafu)?;
        let vectorized = FileExtractor::string_to_vector(&file);
        for object in each_item.range {
            //object.start - 1 is a relatively safe operation, as line number never starts with 0
            let item = &vectorized[object.start - 1..object.end];
            let _catch: Vec<String> =
                FileExtractor::push_to_vector(item, "#[derive(Debug)]".to_string(), true)?;
            //Calling at index 0 because parsed_file consists of a single object
            //Does a recursive check, whether the item is still a valid Rust code
            let parsed_file = &RustItemParser::parse_all_rust_items(&item.join("\n"))?[0];
            let obj_type_to_compare = &parsed_file.object_type().context(CouldNotGetLineSnafu)?;
            let obj_name_to_compare = &parsed_file.object_name().context(CouldNotGetLineSnafu)?;
            if rust_type
                .iter()
                .any(|obj_type| obj_type_to_compare == obj_type)
                && rust_name
                    .iter()
                    .any(|obj_name| obj_name_to_compare == obj_name)
            {
                vecbool.push(true) //present
            } else {
            }
        }
    }
    Ok(vecbool)
}

//Makes an export structure from files
//It takes list of files and processes them into objects that could be worked with
pub fn make_export(filenames: &Vec<PathBuf>) -> Result<Vec<Export>, ErrorHandling> {
    let mut output_vec: Vec<Export> = Vec::new();
    let mut vector_of_changed: Vec<Range<usize>> = Vec::new();
    for filename in filenames {
        let path = env::current_dir()
            .context(InvalidIoOperationsSnafu)?
            .join(filename);

        let parsed_file = RustItemParser::parse_rust_file(&path);
        match parsed_file {
            Ok(value) => {
                for each_object in value {
                    let range = each_object.line_start().context(CouldNotGetLineSnafu)?
                        ..each_object.line_end().context(CouldNotGetLineSnafu)?;
                    vector_of_changed.push(range);
                }
                output_vec.push({
                    Export {
                        filename: path,
                        range: vector_of_changed.to_owned(),
                    }
                });
                vector_of_changed.clear();
            }
            Err(e) => {
                println!("WARNING!\nSKIPPING {e:?} PLEASE REFER TO ERROR LOG");
                continue;
            }
        }
    }
    Ok(output_vec)
}
