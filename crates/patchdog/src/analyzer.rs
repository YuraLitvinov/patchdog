use camino::Utf8Path;
use ra_ap_base_db::RootQueryDb;
use ra_ap_hir::{EditionedFileId, Function, ModuleDefId, Semantics};
use ra_ap_hir_def::{AdtId, FunctionId, db::InternDatabase};
use ra_ap_project_model::{CargoConfig, ProjectManifest, ProjectWorkspace, RustLibSource};

use ra_ap_hir_def::nameres::crate_def_map;
use ra_ap_ide::{RootDatabase, TextRange};
use ra_ap_load_cargo::load_workspace;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice};
use ra_ap_syntax::{AstNode, ast};
use ra_ap_vfs::{AbsPath, Vfs, VfsPath};
use std::panic::AssertUnwindSafe;
use std::{collections::HashMap, env, path::Path};

#[derive(Debug)]
pub struct AnalyzerData {
    pub db: RootDatabase,
    pub vfs: Vfs,
    pub krates: Vec<ra_ap_base_db::Crate>,
}
/// Issue related to RPIT panic https://github.com/rust-lang/rust-analyzer/issues/19339
/// Initializes the Rust Analyzer database and Virtual File System (VFS) for the current project. This function discovers the `Cargo.toml` manifest, loads the workspace, and filters for local crates, returning an `AnalyzerData` struct. This setup is crucial for performing static analysis and code introspection.
///
/// # Returns
/// An `AnalyzerData` struct, containing the initialized `RootDatabase`, `Vfs`, and a vector of `ra_ap_base_db::Crate` instances representing local crates.
pub fn init_analyzer() -> AnalyzerData {
    let absolute = env::current_dir().unwrap();
    let cargo_config = CargoConfig {
        sysroot: Some(RustLibSource::Discover),
        all_targets: true,
        no_deps: true,
        ..Default::default()
    };
    let as_absolute = absolute.join("Cargo.toml");
    let utf8path = AbsPath::assert(Utf8Path::new(as_absolute.to_str().unwrap()));
    let manifest = ProjectManifest::discover_single(utf8path).expect("Couldn't get manifest");
    let no_progress = &|prog| println!("{prog}");
    let workspace = ProjectWorkspace::load(manifest, &cargo_config, no_progress)
        .expect("Couldn't load Cargo.toml");
    let load_cargo_config = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
    };
    let (workspace_db, vfs, _proc_macro) =
        load_workspace(workspace, &cargo_config.extra_env, &load_cargo_config)
            .expect("Couldn't load project");

    let krates = ra_ap_hir::Crate::all(&workspace_db)
        .into_iter()
        .filter_map(|val| {
            if val.origin(&workspace_db).is_local() {
                Some::<ra_ap_base_db::Crate>(val.into())
            } else {
                None
            }
        })
        .collect::<Vec<ra_ap_base_db::Crate>>();
    AnalyzerData {
        db: workspace_db,
        vfs,
        krates,
    }
}

/// Gathers relevant contextual code (functions and structs) from the entire codebase for a specified file and an optional function range. It traverses the `AnalyzerData` to identify local functions and structs, filtering out the target function itself (if `fn_range` is provided) and extracting their source code strings.
///
/// # Arguments
/// * `filepath` - The path to the file for which context is being retrieved.
/// * `fn_range` - An `Option<&TextRange>` specifying a function's range to be excluded from the contextual results.
/// * `analyzer_data` - A reference to the `AnalyzerData` containing the Rust Analyzer database, VFS, and crates.
///
/// # Returns
/// A `HashMap<TextRange, String>` where keys are the text ranges of contextual code elements, and values are their corresponding source code strings.
pub fn contextualizer(
    filepath: &Path,
    fn_range: Option<&TextRange>,
    analyzer_data: &AnalyzerData,
) -> HashMap<TextRange, String> {
    analyzer_data
        .krates
        .iter()
        .flat_map(|krate| {
            crate_def_map(&analyzer_data.db, *krate)
                .modules
                .iter()
                .flat_map(|(_, module)| {
                    module.scope.declarations().flat_map(|decl| {
                        semantic_modules(
                            decl,
                            &analyzer_data.db,
                            fn_range,
                            &analyzer_data.vfs,
                            filepath,
                        )
                    })
                })
        })
        .filter(|(range, _)| {
            // only filter if fn_range was provided
            fn_range != Some(range)
        })
        .collect()
}

fn semantic_modules(
    module: ModuleDefId,
    db: &RootDatabase,
    fn_range: Option<&TextRange>,
    vfs: &Vfs,
    filepath: &Path,
) -> HashMap<TextRange, String> {
    let mut context_strings = HashMap::new();
    match module {
        ra_ap_hir_def::ModuleDefId::FunctionId(fn_id) => {
            let to_path = build_path(fn_id, vfs, db)
                .cloned()
                .unwrap()
                .into_abs_path()
                .unwrap();

            if return_functions(db, fn_id) == fn_range.copied()
                && to_path == Utf8Path::new(filepath.to_str().unwrap())
            {
                let semantics = Semantics::new(db);
                if let Some(sem) = semantics.source::<ra_ap_hir::Function>(fn_id.into()) {
                    let fn_node = sem.value;

                    // Insert the function body itself
                    let range = fn_node.syntax().text_range();
                    let lookup = &db.lookup_intern_function(fn_id).id;
                    let file = lookup.file_id.file_id();
                    if let Some(body) = print_body(file.expect("File ID expected"), range, db) {
                        context_strings.insert(range, body);
                    }

                    // Traverse expressions for name resolution
                    for expr_node in fn_node.syntax().descendants().filter_map(ast::Expr::cast) {
                        match expr_node {
                            ast::Expr::CallExpr(call_expr) => {
                                if let Some(val) = resolve_call_expr(&semantics, &call_expr)
                                    && let Some(body_range) = function_text_range(val.into(), db)
                                    && let Some(body) = print_body(body_range.0, body_range.1, db)
                                {
                                    context_strings.insert(body_range.1, body);
                                }
                            }
                            ast::Expr::MethodCallExpr(method_expr) => {
                                if let Some(val) =
                                    resolve_method_call_expr(&semantics, &method_expr)
                                    && let Some(body_range) = function_text_range(val.into(), db)
                                    && let Some(body) =
                                        print_body(body_range.0, body_range.1, db)
                                {
                                    context_strings.insert(body_range.1, body);
                                }
                                    
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        ra_ap_hir_def::ModuleDefId::AdtId(AdtId::StructId(struct_id)) => {
            let item_struct = db.lookup_intern_struct(struct_id);
            if let Some(file_id) = item_struct.id.file_id.file_id() {
                let actual = vfs.file_path(file_id.file_id(db));
                if *actual
                    == VfsPath::new_real_path(filepath.to_str().unwrap_or_default().to_owned())
                {
                    let range = item_struct.id.to_ptr(db).syntax_node_ptr().text_range();
                    if let Some(body) = print_body(file_id, range, db) {
                        context_strings.insert(range, body);
                    }
                }
            }
        }
        _ => {}
    }
    context_strings
}

// Helper: get the TextRange and file_id of a FunctionId without calling infer
fn function_text_range(
    fn_id: FunctionId,
    db: &RootDatabase,
) -> Option<(EditionedFileId, TextRange)> {
    let lookup = &db.lookup_intern_function(fn_id);
    let as_syn = lookup.id;
    let file_id = as_syn.file_id.file_id()?;
    let range = as_syn.to_ptr(db).syntax_node_ptr().text_range();
    Some((file_id, range))
}

fn resolve_call_expr(sema: &Semantics<'_, RootDatabase>, call: &ast::CallExpr) -> Option<Function> {
    if let Some(path_expr) = call
        .expr()
        .and_then(|e| ast::PathExpr::cast(e.syntax().clone()))
        && let Some(path) = path_expr.path()
    {
        let catch_panic = std::panic::catch_unwind(AssertUnwindSafe(|| sema.resolve_path(&path)));
        if let Some(ra_ap_hir::PathResolution::Def(ra_ap_hir::ModuleDef::Function(f))) =
            catch_panic.ok()?
        {
            return Some(f);
        }
    }
    None
}

fn resolve_method_call_expr(
    sema: &Semantics<'_, RootDatabase>,
    call: &ast::MethodCallExpr,
) -> Option<Function> {
    // The thing being called (`foo` in `foo()`)
    if let Some(path_expr) = call
        .receiver()
        .and_then(|e| ast::PathExpr::cast(e.syntax().clone()))
        && let Some(path) = path_expr.path()
    {
        let catch_panic = std::panic::catch_unwind(AssertUnwindSafe(|| sema.resolve_path(&path)));
        if let Some(ra_ap_hir::PathResolution::Def(ra_ap_hir::ModuleDef::Function(f))) =
            catch_panic.ok()?
        {
            return Some(f);
        }
    }
    None
}

/// Retrieves the Virtual File System (VFS) path associated with a given `FunctionId`. It queries the `RootDatabase` to look up the function's definition and then uses the extracted `FileId` to obtain the corresponding `VfsPath` from the `Vfs`.
///
/// # Arguments
/// * `fn_id` - The `FunctionId` for which to find the file path.
/// * `vfs` - A reference to the `Vfs` (Virtual File System).
/// * `db` - A reference to the `RootDatabase` (Rust Analyzer database).
///
/// # Returns
/// An `Option<&'a VfsPath>` containing a reference to the `VfsPath` if the function's file ID can be successfully resolved, otherwise `None`.
fn build_path<'a>(fn_id: FunctionId, vfs: &'a Vfs, db: &RootDatabase) -> Option<&'a VfsPath> {
    let lookup = &db.lookup_intern_function(fn_id);
    if let Some(file_id) = lookup.id.file_id.file_id() {
        let actual = vfs.file_path(file_id.file_id(db));
        Some(actual)
    } else {
        None
    }
}

fn print_body(file_id: EditionedFileId, range: TextRange, db: &RootDatabase) -> Option<String> {
    let parsed = db.parse(file_id);
    let root = parsed.syntax_node();
    let root_range = root.text_range();
    //Safeguarding against out of bounds ranges
    if !(range.start() >= root_range.start() && range.end() <= root_range.end()) {
        return None;
    }
    let func = parsed.syntax_node().covering_element(range);
    Some(match func {
        ra_ap_syntax::NodeOrToken::Node(n) => n.text().to_string(),
        ra_ap_syntax::NodeOrToken::Token(t) => t.text().to_string(),
    })
}

/// Resolves a given `FunctionId` to its corresponding `TextRange` in the source code, but only if the function is defined within a local crate. It queries the `RootDatabase` to determine the function's origin and, if local, extracts its precise `TextRange`.
///
/// # Arguments
/// * `db` - A reference to the `RootDatabase` (Rust Analyzer database).
/// * `fn_id` - The `FunctionId` of the function to be resolved.
///
/// # Returns
/// An `Option<TextRange>` containing the `TextRange` of the function if it is local and resolvable, otherwise `None`.
fn return_functions(db: &RootDatabase, fn_id: FunctionId) -> Option<TextRange> {
    let lookup = &db.lookup_intern_function(fn_id);
    let as_syn = lookup.id;
    let file_unwrap = as_syn.file_id.file_id();
    if let Some(file_id) = file_unwrap {
        if db.relevant_crates(file_id.file_id(db))[0]
            .data(db)
            .origin
            .is_local()
        {
            let range = as_syn.to_ptr(db).syntax_node_ptr().text_range();
            Some(range)
        } else {
            None
        }
    } else {
        None
    }
}
