use ai_interactions::parse_json::ChangeFromPatch;
use gemini::gemini::{Context, Metadata, Request, SingleFunctionData};
use git_parsing::{Hunk, get_easy_hunk, match_patch_with_parse};
use git2::Diff;
use rayon::prelude::*;
use rust_parsing;
use rust_parsing::ObjectRange;
use rust_parsing::error::ErrorBinding;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use syn::Expr;
use syn::Pat;
use syn::Item;
use std::collections::HashMap;
use std::{
    env, fs,
    ops::Range,
    path::{Path, PathBuf},
};

pub struct FullDiffInfo {
    pub name: String,
    pub object_range: Vec<ObjectRange>,
    pub hunk: Vec<Hunk>,
}
pub struct Difference {
    pub filename: PathBuf,
    pub line: Vec<usize>,
}

pub struct LocalChange {
    filename: PathBuf,
    range: Range<usize>,
    file: String,
}
#[derive(Debug)]
pub struct LocalContext {
    pub context_type: String,
    pub context_name: String
}

pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
    file_exclude: Vec<PathBuf>
) -> Result<Vec<Request>, ErrorBinding> {
    //Collect whole project once, instead of every time for each in function let objects = HashMap<(key, value)>
    //key - (Filename, Option<trait_name> and fn name)
    //value - name and type, body of function
    //Context - FunctionSignature: input args and return types with comment
    //Context - structs as String (as is)
    let tasks: Vec<LocalChange> = exported_from_file
        .par_iter()
        .flat_map(|each| {
            each.range.par_iter().map(move |obj| LocalChange {
                filename: each.filename.clone(),
                range: obj.clone(),
                file: fs::read_to_string(&each.filename).unwrap(),
            })
        })
        .collect();
    let singlerequestdata: Vec<Request> = tasks
        .par_iter()
        .filter_map(|change| {
            //Here we exclude all matching 
            if file_exclude.contains(&change.filename) {
                return None;
            }
            else { 
                let vectorized = FileExtractor::string_to_vector(&change.file);
                let item = &vectorized[change.range.start - 1..change.range.end];
                let parsed_file = RustItemParser::rust_item_parser(&item.join("\n")).ok()?;
                let obj_type_to_compare = parsed_file.names.type_name;
                let obj_name_to_compare = parsed_file.names.name;
                if rust_type.iter().any(|t| &obj_type_to_compare == t)
                    || rust_name.iter().any(|n| &obj_name_to_compare == n)
                {   
                    //At this point in parsed_file we are already aware of all the referenced data    
                    let fn_as_string = item.join("\n");
                    /*
                    Calling find_context(all methods: bla-bla, function: String) -> context(Vec<String>) {
                        1. 
                        2. Find matches in code
                        3. Return matching structures
                    }
                    */       
                    let context = find_context(change, &obj_name_to_compare, &fn_as_string).ok()?;
                    Some(Request {
                        uuid: uuid::Uuid::new_v4().to_string(),
                        data: SingleFunctionData {
                            function_text: fn_as_string,
                            fn_name: obj_name_to_compare,
                            context: context,
                            metadata: Metadata {
                                filepath: change.filename.clone(),
                                line_range: change.range.clone(),
                            },
                        },
                    })
                } else {
                    None
                }
            }
        })
        .collect();
    Ok(singlerequestdata)
}

//Seeking context inside same file, to match probable structures
//Checking uses, to limit amount of crates to be parsed
//Allocating  
//Here importing LocalChange to know path to function, function_text as information to grab
pub fn find_context (change: &LocalChange, fn_name: &str, function_text: &String) -> Result<Context, ErrorBinding> {
    let dir = env::current_dir()?;
    let file = fs::read_to_string(&change.filename)?;
    let parsed = RustItemParser::parse_all_rust_items(&file)?;
    let all_uses: Vec<ObjectRange> = parsed.par_iter().filter_map(|object_range|
        if object_range.names.type_name == "use" {
            Some(object_range.to_owned())
        }
        else {
            None
        }
    ).collect();
    let map_rust_files: HashMap<String, PathBuf> = all_uses.into_iter().map(|each| {
        let name = each.names.name;
        if name == "crate" {
            let mut path = change.filename.clone();
            path.pop();
            return (name.clone(), path)
        }

        let path = dir.join(&name);

        if path.exists() {
            return (name, path);
        }
        return ("".to_string(), Path::new("").to_path_buf());
    }).collect();
    let context_files = map_rust_files.into_values().map(|val|val).collect::<Vec<PathBuf>>();
    let mut rust_files=  vec![];
    let tokens = syn::parse_file(&function_text).unwrap().items;
    //Objects variable here contains function, struct calls from inside the documented function
    //Data variable contains all fn and struct objects from inside those crates, that have to be matched with objects variable
    let objects = grep_objects(tokens);

    let mut data = vec![];
    for each in context_files {
        find_rust_files(each, &mut rust_files);
    }
    for object in rust_files {
        if let Ok(ok) = fs::read_to_string(object) {
            let objs = RustItemParser::parse_all_rust_items(&ok)?;
            for each in objs {
                if each.names.type_name == "struct" || each.names.type_name == "fn" {
                    data.push(each);
                }
            }
        }
    }
    let map_objects_in_fn = objects.par_iter().map(|obj|
        (
        (   obj.context_type.clone(),
            obj.context_name.clone()
        ), 
        "".to_string()
    )
    )
        .collect::<HashMap<(String, String), String>>();
    let map_data = data.par_iter().map(|obj|
        ((obj.names.type_name.clone(), obj.names.name.clone()),
        obj.clone())
    )
        .collect::<HashMap<(String, String), ObjectRange>>();
    //Here matching found objects inside functions and within used crates in the file with function
    let matches: Vec<ObjectRange> = map_data.clone().into_iter()
        .filter_map(|(key, val)| if map_objects_in_fn.contains_key(&key) { 
            return Some(val);
        } else {
            None
        } )
        .collect();
    println!("{}\n{:#?}", fn_name, matches);
    //println!("{:#?}", map_data);
    println!("{:#?}", map_objects_in_fn);

 //Now we want to find all use statements, to make scope of search smaller. 
    //Instead of parsing whole project - we parse few of the crates
    Ok(Context {
        class_name: "".to_string(),
        external_dependecies: vec!["".to_string()],
        old_comment: vec!["".to_string()],
    })
}

fn find_rust_files(dir: PathBuf, rust_files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_rust_files(path, rust_files); // Recurse into subdirectory
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    rust_files.push(path);
                }
            }
        }
    }
}

/// Retrieves parsed patch data from a specified patch file.
/// It constructs the absolute path to the patch file and the relative working directory,
/// then calls `get_patch_data` to process the patch.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` indicating the path to the patch file.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>`:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector containing details of changes extracted from the patch.
/// - `Err(ErrorBinding)`: If the current directory cannot be determined or patch data extraction fails.
pub fn patch_data_argument(path_to_patch: PathBuf) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let path = env::current_dir()?;
    let patch = get_patch_data(path.join(path_to_patch), path)?;
    Ok(patch)
}

/*
Pushes information from a patch into vector that contains lines
at where there are unique changed objects reprensented with range<usize>
and an according path each those ranges that has to be iterated only once
*/
/// Processes a Git patch to identify specific code changes within Rust files.
/// It first exports the raw changes from the patch, then iterates through these differences.
/// For each difference, it parses the corresponding Rust file and identifies actual Rust items (functions, structs, etc.)
/// whose line ranges overlap with the reported changes in the patch.
/// The result is a refined list of `ChangeFromPatch` objects containing only relevant Rust item ranges.
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` indicating the path to the patch file.
/// * `relative_path` - A `PathBuf` representing the relative base path from which files are referenced.
///
/// # Returns
///
/// A `Result<Vec<ChangeFromPatch>, ErrorBinding>`:
/// - `Ok(Vec<ChangeFromPatch>)`: A vector of `ChangeFromPatch` objects, each containing a filename and a vector of `Range<usize>` indicating the line ranges of identified Rust items that were changed by the patch.
/// - `Err(ErrorBinding)`: If patch export fails or Rust file parsing encounters an error.
pub fn get_patch_data(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<ChangeFromPatch>, ErrorBinding> {
    let export = patch_export_change(path_to_patch, relative_path)?;
    let export_difference = export
        .par_iter()
        .flat_map(|difference| {
            let parsed = RustItemParser::parse_rust_file(&difference.filename).ok()?;
            let vector_of_changed = parsed
                .par_iter()
                .flat_map(|each_parsed| {
                    let range = each_parsed.line_start()..each_parsed.line_end();
                    if difference.line.par_iter().any(|line| range.contains(line)) {
                        Some(range)
                    } else {
                        None
                    }
                })
                .collect();
            Some(ChangeFromPatch {
                range: vector_of_changed,
                filename: difference.filename.to_owned(),
            })
        })
        .collect();
    Ok(export_difference)
}

/// Stores detailed information about changed objects in a Git patch.
/// It parses the raw patch content, matches patch hunks with parsed Rust items, and for each changed file,
/// it extracts relevant hunks and parses the file's Rust items to provide a comprehensive `FullDiffInfo`.
///
/// # Arguments
///
/// * `relative_path` - A reference to a `Path` indicating the base directory for relative file paths.
/// * `patch_src` - A byte slice (`&[u8]`) containing the raw content of the patch file.
///
/// # Returns
///
/// A `Result<Vec<FullDiffInfo>, ErrorBinding>`:
/// - `Ok(Vec<FullDiffInfo>)`: A vector of `FullDiffInfo` structs, each containing the filename, parsed object ranges within that file, and associated hunks.
/// - `Err(ErrorBinding)`: If diff parsing, hunk matching, file reading, or Rust item parsing fails.
fn store_objects(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, ErrorBinding> {
    let diff = Diff::from_buffer(patch_src).unwrap();
    let changes = &match_patch_with_parse(relative_path, &diff)?;
    let vec_of_surplus = changes
        .iter()
        .map(|change| {
            let list_of_unique_files = get_easy_hunk(&diff, &change.filename()).unwrap();
            let path = relative_path.join(change.filename());
            let file = fs::read_to_string(&path).unwrap();
            let parsed = RustItemParser::parse_all_rust_items(&file).unwrap();
            FullDiffInfo {
                name: change.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            }
        })
        .collect();
    Ok(vec_of_surplus)
}

/// Exports detailed line differences from a Git patch, focusing on identifying lines that introduce changes within Rust code objects.
/// It reads the patch file, extracts general diff information, then iterates through each changed hunk.
/// For each hunk, it reads the corresponding file, parses its Rust items, and checks if the changed lines within the hunk fall outside of existing, valid Rust objects (indicating new or significantly altered structures).
///
/// # Arguments
///
/// * `path_to_patch` - A `PathBuf` specifying the path to the patch file.
/// * `relative_path` - A `PathBuf` specifying the base directory for relative file paths.
///
/// # Returns
///
/// A `Result<Vec<Difference>, ErrorBinding>`:
/// - `Ok(Vec<Difference>)`: A vector of `Difference` structs, each containing the filename and a list of line numbers that represent changes not neatly contained within existing Rust objects.
/// - `Err(ErrorBinding)`: If file reading, object storage, or Rust item parsing fails.
fn patch_export_change(
    path_to_patch: PathBuf,
    relative_path: PathBuf,
) -> Result<Vec<Difference>, ErrorBinding> {
    let mut change_in_line: Vec<usize> = Vec::new();
    let mut line_and_file: Vec<Difference> = Vec::new();
    let patch_text = fs::read(&path_to_patch)?;
    let each_diff = store_objects(&relative_path, &patch_text)?;
    for diff_hunk in &each_diff {
        let path_to_file = relative_path.to_owned().join(&diff_hunk.name);
        let file = fs::read_to_string(&path_to_file)?;
        let parsed = RustItemParser::parse_all_rust_items(&file)?;
        let path = path_to_file;

        for each in &diff_hunk.hunk {
            let parsed_in_diff = &parsed;
            if FileExtractor::check_for_valid_object(parsed_in_diff, each.get_line())? {
                continue;
            }
            change_in_line.push(each.get_line());
        }
        line_and_file.push(Difference {
            filename: path,
            line: change_in_line.to_owned(),
        });
        change_in_line.clear();
    }
    Ok(line_and_file)
}

pub fn grep_objects(tokens: Vec<Item>) -> Vec<LocalContext> {
    let mut objects: Vec<LocalContext> = Vec::new();
        for token in tokens {        
        match token {
            Item::Fn(f) => read_block(*f.block, &mut objects),
            _ => &mut objects
        };
    }
    objects
}

fn read_block(block: syn::Block, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    for stmt in block.stmts {
        match stmt {
            syn::Stmt::Expr(e,_) => {
                match_expr(e, objects);
            },
            syn::Stmt::Local(local) => {
                handle_pat(local.pat, objects);
                match_expr(*local.init.unwrap().expr, objects);
            }
            _ => {}
        }
    }
    objects
}

fn match_expr(expr: Expr, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    match expr {
        Expr::Assign(assign) => {
            match_expr(*assign.left, objects);
            match_expr(*assign.right, objects)
        },
        Expr::Block(block) => read_block(block.block, objects),
        Expr::Call(call) => {
            match_expr(*call.func, objects);
            for arg in call.args {
                match_expr(arg, objects);
            }
            objects
        }
        Expr::Closure(closure) => match_expr(*closure.body, objects),
        Expr::ForLoop(for_loop) => {
            handle_pat(*for_loop.pat, objects);
            match_expr(*for_loop.expr, objects);
            read_block(for_loop.body, objects)
        }
        Expr::If(if_expr) => {
            match_expr(*if_expr.cond, objects);
            read_block(if_expr.then_branch, objects);

            if let Some((_, else_expr)) = if_expr.else_branch {
                match_expr(*else_expr, objects);
            } else {
                objects.push(LocalContext { context_type: "".to_string(), context_name: "".to_string() });
            }
            objects
        }
        Expr::Let(let_expr) => {
            match_expr(*let_expr.expr, objects);
            handle_pat(*let_expr.pat, objects)
        }
        Expr::Loop(loop_expr) => read_block(loop_expr.body, objects),
        Expr::MethodCall(method_call) => handle_method_call(method_call, objects),
        Expr::Struct(strukt) => handle_struct(strukt, objects),
        Expr::Path(path_expr) => handle_path(path_expr, objects),
        Expr::Try(try_expr) => match_expr(*try_expr.expr, objects),
        Expr::TryBlock(try_block) => read_block(try_block.block, objects),
        Expr::Unsafe(unsafe_expr) => read_block(unsafe_expr.block, objects),
        Expr::While(while_expr) => {
            match_expr(*while_expr.cond, objects);
            read_block(while_expr.body, objects)
        }
        _ => objects
    }
}


fn handle_pat(pat: Pat, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    match pat {
        Pat::Struct(ps) => {
            for field in ps.fields {
                handle_pat(*field.pat, objects);
            }
        }
        _ => {}
    }
    objects
}

fn handle_method_call(method_call: syn::ExprMethodCall, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    match_expr(*method_call.receiver, objects);
    for arg in method_call.args {
        match_expr(arg, objects);
    }
    objects.push(LocalContext { context_type: "fn".to_string(), context_name: method_call.method.to_string() });
    objects
}

fn handle_struct(strukt: syn::ExprStruct, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    let some = strukt.path.get_ident().map(|ident| ident.to_string());
    objects.push(LocalContext { context_type: "struct".to_string(), context_name: some.unwrap_or("".to_string()) });
    objects
}

fn handle_path(path_expr: syn::ExprPath, objects: &mut Vec<LocalContext>) -> &mut Vec<LocalContext> {
    let segment = path_expr.path.segments.into_iter().map(|segment|
        segment.ident.to_string()
    ).collect::<Vec<String>>();
    let _ = segment.into_iter().map(|each| objects.push(LocalContext { context_type: "path".to_string(), context_name: each }));
    objects
}

