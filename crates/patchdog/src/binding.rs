use ai_interactions::parse_json::ChangeFromPatch;
use ai_interactions::return_prompt;
use clap::error::Result;
use gemini::request_preparation::{Context, Metadata, Request, SingleFunctionData};
use git_parsing::{get_easy_hunk, match_patch_with_parse, Git2ErrorHandling, Hunk};
use git2::Diff;
use rayon::prelude::*;
use rust_parsing::{self, ErrorHandling};
use rust_parsing::ObjectRange;
use rust_parsing::error::ErrorBinding;
use rust_parsing::file_parsing::{FileExtractor, Files};
use rust_parsing::rust_parser::{RustItemParser, RustParser};
use serde::{Deserialize, Serialize};
use syn::{UseTree};
use syn::Item;
use std::collections::HashMap;
use std::{
    env, fs,
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    pub ident: String,
    pub module: String,
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathObject {
    pub filename: PathBuf,
    pub object: ObjectRange
}

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
    pub filename: PathBuf,
    pub range: Range<usize>,
    pub file: String,
}
#[derive(Debug, Clone)]
pub struct LocalContext {
    pub context_type: String, 
    pub context_name: String,  
    pub context_path: String,  
}

fn file_belongs_to_dir(file: &Path, dir: &Path) -> std::io::Result<bool> {
    let file_path = fs::canonicalize(file)?;
    let dir_path = fs::canonicalize(dir)?;
    Ok(file_path.starts_with(&dir_path))
}

fn is_file_allowed(file: &Path, exclusions: &[PathBuf]) -> std::io::Result<bool> {
    for dir in exclusions {
        if file_belongs_to_dir(file, dir)? {
            return Ok(false);
        }
    }
    Ok(true) // not in any excluded dir
}


pub fn changes_from_patch(
    exported_from_file: Vec<ChangeFromPatch>,
    rust_type: Vec<String>,
    rust_name: Vec<String>,
    file_exclude: &[PathBuf]
) -> Result<Vec<Request>, ErrorBinding> {
    //Collect whole project once, instead of every time for each in function let objects = HashMap<(key, value)>
    //key - (Filename, Option<trait_name> and fn name)
    //value - name and type, body of function
    //Context - FunctionSignature: input args and return types with comment
    //Context - structs as String (as is)
    //Collecting tasks separately to avoid filesystem overhead
    let tasks: Vec<LocalChange> = exported_from_file
        .par_iter()
        .flat_map(|each| {
            each.range.par_iter().filter_map(move |obj| Some(LocalChange {
                filename: each.filename.clone(),
                range: obj.clone(),
                file: fs::read_to_string(&each.filename).ok()?,
            }))
        })
        .collect();
    let singlerequestdata: Vec<Request> = tasks
        .par_iter()
        .filter_map(|change| {
            //Here we only allow files, that are not in the config.yaml-Patchdog_settings-excluded_files
            if is_file_allowed(&change.filename, file_exclude).ok()? {
                let vectorized = FileExtractor::string_to_vector(&change.file);
                let item = &vectorized[change.range.start - 1..change.range.end];
                let parsed_file = RustItemParser::rust_item_parser(&item.join("\n")).ok()?;
                let obj_type_to_compare = parsed_file.names.type_name;
                let obj_name_to_compare = parsed_file.names.name;
                if rust_type.par_iter().any(|t| &obj_type_to_compare == t)
                    || rust_name.par_iter().any(|n| &obj_name_to_compare == n)
                && return_prompt().ok()?.patchdog_settings.excluded_functions.contains(&obj_name_to_compare) {   
                    //At this point in parsed_file we are already aware of all the referenced data    
                    let fn_as_string = item.join("\n");
                    /*
                    Calling find_context(all methods: bla-bla, function: String) -> context(Vec<String>) {
                        1. 
                        2. Find matches in code
                        3. Return matching structures
                    }
                    */       
                    let context = find_context(change.filename.to_owned(), &obj_name_to_compare, &fn_as_string).ok()?;
                    Some(Request {
                        uuid: uuid::Uuid::new_v4().to_string(),
                        data: SingleFunctionData {
                            function_text: fn_as_string,
                            fn_name: obj_name_to_compare,
                            context,
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
            else { 
                None
            }
        })
        .collect();
    Ok(singlerequestdata)
}

//Seeking context inside same file, to match probable structures
//Checking uses, to limit amount of crates to be parsed
//Instead of parsing whole project - we parse few of the crates
///   Identifies and extracts the source code of external Rust dependencies for a specified function within a project.
///   It first parses `use` statements from the file at `change`, then collects all related Rust files, and subsequently extracts code segments from these files that are referenced within the provided `function_text`.
///   The function returns a `Context` object, containing a list of strings where each string represents the source code of a detected external dependency, excluding the function itself.
pub fn find_context (change: PathBuf, fn_name: &str, function_text: &str) -> Result<Context, ErrorHandling> {
    let mut context = vec![];
    //Crate level-context seeking
    let file = fs::read_to_string(&change)?;
    let parsed = syn::parse_file(&file)?;
    //Here we try to find the paths used in the certain file, if they are within the project, then,
    //We can reach them and get the context from there
    //Hashmaps doesn't allow matching certain crate more than once
    //Now we want to find all use statements, to make scope of search smaller. 
    let all_uses = parse_use(parsed.items);
    //Here in map_rust_files we locate all the crates used within the file where function is located
    let map_rust_files = collect_paths(all_uses.clone(), change.clone())?;
    let use_map = all_uses.par_iter().map(|single_use| 
        (single_use.object.to_owned(), single_use.to_owned()) 
    ).collect::<HashMap<String, UseItem>>();
    let mut paths = vec![];
    paths.push(change.to_owned());
    for each in &map_rust_files {
        find_rust_files(each.0.to_path_buf(), &mut paths);
    }
    let assorted_paths = paths.into_iter().filter_map(|path| 
        Some((path.file_stem()?.to_str()?.to_string(), path))
    ).collect::<HashMap<String, PathBuf>>();
    for path in assorted_paths.clone() {
        let file = fs::read_to_string(&path.1)?;
        let parsed = RustItemParser::parse_all_rust_items(&file)?;
        for parse in parsed {
            if use_map.contains_key(&parse.names.name) || use_map.contains_key("self") || use_map.contains_key("*") {
                context.push(PathObject { filename: path.1.to_owned(), object: parse });
            }
        }        
    }

    //Here we want to match the function with all the imports
    let parsed_function = syn::parse_file(function_text)?;
    let tokens = grep_objects(parsed_function.items);
    let map_function = tokens.par_iter().filter_map(|con| 
        if use_map.contains_key(&con.context_name) || (con.context_name == con.context_path) {
            Some((con.context_path.to_owned(), con.to_owned()))
        }else {
            None
        }
    ).collect::<HashMap<String, LocalContext>>();
    let map_filedata = match_context(context.clone());
    let context = map_filedata.clone()
        .into_iter()
        .filter_map(|(key, value_function)| {
            if map_function.contains_key(&key) {
                if fn_name != key {
                    Some(value_function.to_owned())
                }
                else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<PathObject>>();
    let dependencies = context
    .into_par_iter()
    .filter_map(|dep| {
        let a = fs::read_to_string(&dep.filename).ok()?;
        let to_vec = FileExtractor::string_to_vector(&a)
            [dep.object.line_start()-1..dep.object.line_end()]
            .join("\n");
        Some(to_vec)
    }).collect::<Vec<String>>();
    Ok(Context {
        class_name: "".to_string(),
        external_dependencies: dependencies,
        old_comment: vec!["".to_string()],
    })
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

fn grep_objects(tokens: Vec<syn::Item>) -> Vec<LocalContext> {
    let mut objects: Vec<LocalContext> = Vec::new();
    for token in tokens {        
        if let syn::Item::Fn(f) = token { 
            read_block(*f.block, &mut objects, "".to_string()); 
        }
    }
    objects
}

// Returns hashmap over uses within the files
fn match_context(context: Vec<PathObject>) -> HashMap<String, PathObject> {
    context.par_iter().map(|context| 
        (context.object.names.name.clone(), PathObject { filename: context.filename.to_owned(), object: context.object.to_owned()})
    ) .collect::<HashMap<String, PathObject>>()
}

fn collect_paths(all_uses: Vec<UseItem>, filename: PathBuf) -> Result<HashMap<PathBuf, String>, ErrorHandling> { 
        let dir = env::current_dir()?;
        let map_rust_files: HashMap<PathBuf, String> = all_uses.clone().into_iter().filter_map(|each| {
        let name = each.ident;
        if name == "crate" {
            let mut path = filename.clone();
            path.pop();
            return Some((path, name.clone()))
        }
        let read_dir = dir.read_dir().ok()?;
        let entries = read_dir.into_iter().filter_map(|entry| 
            if entry.as_ref().ok()?.path().join(&name).exists() {
                Some((entry.ok()?.path().join(&name), name.clone()))
            }
            else {
                None
            }
        ).collect::<HashMap<PathBuf, String>>();
        if let Some(each) = entries.into_iter().next() {
            return Some(each);
        }
        None
    }).collect();
    Ok(map_rust_files)
}

fn find_rust_files(dir: PathBuf, rust_files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_rust_files(path, rust_files); // Recurse into subdirectory
            } else if let Some(ext) = path.extension() && ext == "rs" {
                rust_files.push(path);
            }
        }
    }
}

pub fn parse_use(tokens: Vec<Item>) -> Vec<UseItem> {
    let mut items = Vec::new();

    for token in tokens {
        if let Item::Use(u) = token {
            flatten_tree(&u.tree, "".to_string(), "".to_string(), &mut items);
        }
    }

    items
}

fn flatten_tree(tree: &UseTree, ident: String, module: String, acc: &mut Vec<UseItem>) {
    match tree {
        UseTree::Path(path) => {
            let new_ident = if ident.is_empty() { path.ident.to_string() } else { ident.clone() };
            let new_module = if module.is_empty() { path.ident.to_string() } else { format!("{}", path.ident) };
            flatten_tree(&path.tree, new_ident, new_module, acc);
        }
        UseTree::Name(name) => {
            acc.push(UseItem {
                ident: ident.clone(),
                module: module.clone(),
                object: name.ident.to_string(),
            });
        }
        UseTree::Rename(rename) => {
            acc.push(UseItem {
                ident: ident.clone(),
                module: module.clone(),
                object: rename.ident.to_string(),
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_tree(item, ident.clone(), module.clone(), acc);
            }
        }
        UseTree::Glob(_) => {
            acc.push(UseItem {
                ident: ident.clone(),
                module: module.clone(),
                object: "*".to_string(),
            });
        }
    }
}

fn store_objects(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<FullDiffInfo>, Git2ErrorHandling> {
    let diff = Diff::from_buffer(patch_src)?;
    let changes = &match_patch_with_parse(relative_path, &diff)?;
    let vec_of_surplus = changes
        .iter()
        .filter_map(|change| {
            let list_of_unique_files = get_easy_hunk(&diff, &change.filename()).ok()?;
            let path = relative_path.join(change.filename());
            let file = fs::read_to_string(&path).ok()?;
            let parsed = RustItemParser::parse_all_rust_items(&file).ok()?;
            Some(FullDiffInfo {
                name: change.filename(),
                object_range: parsed,
                hunk: list_of_unique_files,
            })
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

fn read_block(block: syn::Block, objects: &mut Vec<LocalContext>, parent_path: String) {
    for stmt in block.stmts {
        match stmt {
            syn::Stmt::Expr(e,_) => {
                match_expr(e, objects, parent_path.clone());
            },
            syn::Stmt::Local(local) => {
                handle_pat(local.pat, objects, parent_path.clone());
                if let Some(init_expr) = local.init {
                    match_expr(*init_expr.expr, objects, parent_path.clone());
                }
            }
            _ => {}
        }
    }
}

fn match_expr(expr: syn::Expr, objects: &mut Vec<LocalContext>, parent_path: String) {
    match expr {
        syn::Expr::Assign(assign) => {
            match_expr(*assign.left, objects, parent_path.clone());
            match_expr(*assign.right, objects, parent_path);
        },
        syn::Expr::Block(block) => read_block(block.block, objects, parent_path),
        syn::Expr::Call(call) => {
            match_expr(*call.func, objects, parent_path.clone());
            for arg in call.args {
                match_expr(arg, objects, parent_path.clone());
            }
        }
        syn::Expr::Closure(closure) => match_expr(*closure.body, objects, parent_path),
        syn::Expr::ForLoop(for_loop) => {
            handle_pat(*for_loop.pat, objects, parent_path.clone());
            match_expr(*for_loop.expr, objects, parent_path.clone());
            read_block(for_loop.body, objects, parent_path);
        }
        syn::Expr::If(if_expr) => {
            match_expr(*if_expr.cond, objects, parent_path.clone());
            read_block(if_expr.then_branch, objects, parent_path.clone());

            if let Some((_, else_expr)) = if_expr.else_branch {
                match_expr(*else_expr, objects, parent_path);
            }
        }
        syn::Expr::Let(let_expr) => {
            match_expr(*let_expr.expr, objects, parent_path.clone());
            handle_pat(*let_expr.pat, objects, parent_path);
        }
        syn::Expr::Loop(loop_expr) => read_block(loop_expr.body, objects, parent_path),
        syn::Expr::MethodCall(method_call) => handle_method_call(method_call, objects, parent_path),
        syn::Expr::Struct(strukt) => handle_struct(strukt, objects, parent_path),
        syn::Expr::Path(path_expr) => handle_path(path_expr, objects, parent_path),
        syn::Expr::Try(try_expr) => match_expr(*try_expr.expr, objects, parent_path),
        syn::Expr::TryBlock(try_block) => read_block(try_block.block, objects, parent_path),
        syn::Expr::Unsafe(unsafe_expr) => read_block(unsafe_expr.block, objects, parent_path),
        syn::Expr::While(while_expr) => {
            match_expr(*while_expr.cond, objects, parent_path.clone());
            read_block(while_expr.body, objects, parent_path);
        }
        _ => {}
    }
}

fn handle_pat(pat: syn::Pat, _objects: &mut Vec<LocalContext>, _parent_path: String) {
    if let syn::Pat::Struct(ps) = pat {
        for field in ps.fields {
            handle_pat(*field.pat, _objects, _parent_path.clone());
        }
    }
}

fn handle_method_call(method_call: syn::ExprMethodCall, objects: &mut Vec<LocalContext>, parent_path: String) {
    let full_path = if parent_path.is_empty() {
        method_call.method.to_string()
    } else {
        format!("{}::{}", parent_path, method_call.method)
    };

    match_expr(*method_call.receiver, objects, parent_path.clone());

    for arg in method_call.args {
        match_expr(arg, objects, parent_path.clone());
    }

    objects.push(LocalContext {
        context_type: "fn".to_string(),
        context_name: method_call.method.to_string(),
        context_path: full_path,
    });
}

fn handle_struct(strukt: syn::ExprStruct, objects: &mut Vec<LocalContext>, parent_path: String) {
    let ident = strukt
        .path
        .get_ident()
        .into_iter()
        .filter_map(|i| Some(i.to_string()))
        .collect::<String>();
    let full_path = if parent_path.is_empty() { 
        ident.clone() 
    } else { 
        format!("{}::{}", parent_path, ident) 
    };
    objects.push(LocalContext {
        context_type: "struct".to_string(),
        context_name: ident,
        context_path: full_path,
    });
}

fn handle_path(
    path_expr: syn::ExprPath,
    objects: &mut Vec<LocalContext>,
    _parent_path: String,
) {
    let segments: Vec<String> = path_expr
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();

    if segments.is_empty() {
        return;
    }

    let (_, last_name) = if segments.len() > 1 {
        (
            segments[..segments.len() - 1].join("::"),
            segments.last().unwrap().to_string(),     
        )
    } else {
        (String::new(), segments.first().unwrap().clone())
    };

    objects.push(LocalContext {
        context_type: "path".to_string(),
        context_name: segments.first().unwrap().to_owned(),      
        context_path: last_name,     
    });
}
