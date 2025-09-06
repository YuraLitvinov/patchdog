use camino::Utf8Path;
use la_arena::Idx;
use ra_ap_base_db::RootQueryDb;
use ra_ap_hir::{
    EditionedFileId, ModuleDefId,
    db::{DefDatabase, HirDatabase},
};
use ra_ap_hir_def::{
    AdtId, DefWithBodyId, FunctionId,
    db::InternDatabase,
    expr_store::HygieneId,
    resolver::{HasResolver, ValueNs},
};
use ra_ap_project_model::{CargoConfig, ProjectManifest, ProjectWorkspace, RustLibSource};

#[allow(unused)]
use ra_ap_hir_def::{
    expr_store::Body,
    hir::Expr::{self, *},
    lang_item::LangItemTarget,
    nameres::crate_def_map,
};
use ra_ap_ide::{AnalysisHost, RootDatabase, TextRange};
use ra_ap_load_cargo::load_workspace;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice};
use ra_ap_vfs::{AbsPath, Vfs, VfsPath};
use std::{
    collections::{HashMap, HashSet},
    env,
    path::Path,
};

#[derive(Debug)]
pub struct AnalyzerData {
    pub db: RootDatabase,
    pub vfs: Vfs,
    pub krates: Vec<ra_ap_base_db::Crate>,
}

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
    let workspace = ProjectWorkspace::load(manifest, &cargo_config, no_progress).unwrap();
    let load_cargo_config = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
    };
    let (workspace_db, vfs, _proc_macro) =
        load_workspace(workspace, &cargo_config.extra_env, &load_cargo_config).unwrap();
    let host = AnalysisHost::with_database(workspace_db);
    let db = host.raw_database().to_owned();
    let krates = ra_ap_hir::Crate::all(&db)
        .into_iter()
        .filter_map(|val| {
            if val.origin(&db).is_local() {
                Some::<ra_ap_base_db::Crate>(val.into())
            } else {
                None
            }
        })
        .collect::<Vec<ra_ap_base_db::Crate>>();
    AnalyzerData { db, vfs, krates }
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
        .clone()
        .into_iter()
        .flat_map(|krate| {
            let defmap = &crate_def_map(&analyzer_data.db, krate).modules;
            defmap.iter().flat_map(|(_, module)| {
                module.scope.declarations().flat_map(|decl| {
                    match_modules(
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
/// Recursively processes a `ModuleDefId` to extract source code strings for functions and structs relevant to a specified file. It identifies dependencies, filters for local code, and collects the source text of elements that are not the target function itself (if provided). This function is key to building contextual understanding of a code change.
///
/// # Arguments
/// * `module` - The `ModuleDefId` representing the module or item to analyze.
/// * `db` - A reference to the `RootDatabase` for Rust Analyzer operations.
/// * `fn_range` - An `Option<&TextRange>` for a specific function range to be excluded from context.
/// * `vfs` - A reference to the `Vfs` (Virtual File System).
/// * `filepath` - The `Path` of the file being analyzed.
///
/// # Returns
/// A `HashMap<TextRange, String>` where keys are the text ranges of contextual code elements (functions or structs) and values are their corresponding source code strings.
fn match_modules(
    module: ModuleDefId,
    db: &RootDatabase,
    fn_range: Option<&TextRange>,
    vfs: &Vfs,
    filepath: &Path,
) -> HashMap<TextRange, String> {
    let mut context_strings = HashMap::new();
    match module {
        ra_ap_hir_def::ModuleDefId::FunctionId(fn_id) => {
            let defwith = DefWithBodyId::from(fn_id);
            let to_path = build_path(fn_id, vfs, db)
                .cloned()
                .unwrap()
                .into_abs_path()
                .unwrap();
            if return_functions(db, fn_id) == fn_range.copied()
                && to_path == Utf8Path::new(filepath.to_str().unwrap())
            {
                let body = db.body(defwith);
                let mut expressions = HashSet::new();
                let mut fn_deps = HashSet::new();
                for each in body.exprs() {
                    seek_dependencies(&mut expressions, each.0, &body);
                }
                for each in expressions {
                    let expr = &body[each.to_owned()];
                    let range = match_expressions(&mut fn_deps, expr, db, defwith, each);
                    let lookup = &db.lookup_intern_function(fn_id).id;
                    let file = lookup.file_id.file_id();
                    if let Some(val) = range
                        && let Some(edfileid) = file
                    {
                        let body = print_body(edfileid, val, db);
                        context_strings.insert(val, body);
                    }
                }
            }
        }
        //AdtId=Struct, TraitId and TraitAliasId
        ra_ap_hir_def::ModuleDefId::AdtId(AdtId::StructId(struct_id)) => {
            let item_struct = db.lookup_intern_struct(struct_id);
            let file_unwrap = item_struct.id.file_id.file_id();
            if let Some(file_id) = file_unwrap {
                let actual = vfs.file_path(file_id.file_id(db));
                if *actual
                    == VfsPath::new_real_path(filepath.to_str().unwrap_or_default().to_owned())
                {
                    let range = item_struct.id.to_ptr(db).syntax_node_ptr().text_range();
                    let body = print_body(file_id, range, db);
                    context_strings.insert(range, body);
                }
            }
        }
        _ => (),
    }
    context_strings
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
    let as_syn = lookup.id;
    let file_unwrap = as_syn.file_id.file_id();
    if let Some(file_id) = file_unwrap {
        let actual = vfs.file_path(file_id.file_id(db));
        Some(actual)
    } else {
        None
    }
}

/// Extracts and returns the raw source code text corresponding to a specific `TextRange` within a given file. It parses the file using the `RootDatabase` and then navigates the syntax tree to locate the syntax node or token covering the specified range, converting its text content into a `String`.
///
/// # Arguments
/// * `file_id` - The `EditionedFileId` of the file to retrieve the text from.
/// * `range` - The `TextRange` specifying the exact portion of the file's content to extract.
/// * `db` - A reference to the `RootDatabase` for parsing file content.
///
/// # Returns
/// A `String` containing the source code text found within the specified `TextRange`.
fn print_body(file_id: EditionedFileId, range: TextRange, db: &RootDatabase) -> String {
    let to_source = db.parse(file_id);
    let func = to_source.syntax_node().covering_element(range);
    match func {
        ra_ap_syntax::NodeOrToken::Node(n) => n.text().to_string(),
        ra_ap_syntax::NodeOrToken::Token(t) => t.text().to_string(),
    }
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

/// Analyzes a given expression within a function body to identify and collect `FunctionId`s of any referenced functions, including method calls and path expressions. It leverages the Rust Analyzer's inference and resolution capabilities to find the `TextRange` of the resolved function, adding its `FunctionId` to a mutable set.
///
/// # Arguments
/// * `fn_ids` - A mutable `HashSet<FunctionId>` to accumulate discovered function IDs.
/// * `expression` - A reference to the `Expr` (expression) to be analyzed.
/// * `db` - A reference to the `RootDatabase` for Rust Analyzer operations.
/// * `body_id` - The `DefWithBodyId` of the function or constant body containing the expression.
/// * `idx_expr` - The `Idx<Expr>` of the expression within its body.
///
/// # Returns
/// An `Option<TextRange>` representing the `TextRange` of the resolved function if it is local and found, otherwise `None`.
fn match_expressions(
    fn_ids: &mut HashSet<FunctionId>,
    expression: &Expr,
    db: &RootDatabase,
    body_id: DefWithBodyId,
    idx_expr: Idx<Expr>,
) -> Option<TextRange> {
    let res = body_id.resolver(db);
    let syntax_context = ra_ap_span::SyntaxContext::root(ra_ap_span::Edition::Edition2024);
    let hygiene = HygieneId::new(syntax_context);
    let infer = db.infer(body_id);
    let k = infer.assoc_resolutions_for_expr(idx_expr);
    if let Some(val) = k
        && let ra_ap_hir_def::AssocItemId::FunctionId(f) = val.0
    {
        fn_ids.insert(f);
        return_functions(db, f);
    }

    let receiver = infer.method_resolution(idx_expr);
    if let Some(val) = receiver {
        fn_ids.insert(val.0);
        return_functions(db, val.0);
    }
    if let Expr::Path(p) = expression {
        let per = res.resolve_path_in_value_ns_fully(db, p, hygiene);
        if let Some(val) = per
            && let ValueNs::FunctionId(f) = val
        {
            fn_ids.insert(f);
            return_functions(db, f)
        } else {
            None
        }
    } else {
        None
    }
}

/// Recursively traverses the Abstract Syntax Tree (AST) of a given expression within a function body, collecting all `ExprId`s that represent sub-expressions or calls. This function helps in identifying and tracking all expressions relevant to a function's logic, including those nested in control flow structures like `if`, `loop`, `call`, `method_call`, and `match`.
///
/// # Arguments
/// * `expressions` - A mutable `HashSet<ra_ap_hir_def::hir::ExprId>` to store the IDs of all encountered expressions.
/// * `expr_id` - The starting `ExprId` for the recursive traversal.
/// * `body_match` - A reference to the `Body` of the function or constant containing the expressions.
///
/// # Returns
/// This function modifies the `expressions` `HashSet` in place and does not return a value.
fn seek_dependencies(
    expressions: &mut std::collections::HashSet<ra_ap_hir_def::hir::ExprId>,
    expr_id: ra_ap_hir_def::hir::ExprId,
    body_match: &Body,
) {
    expressions.insert(expr_id);

    let expr = &body_match[expr_id];
    match expr {
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            seek_dependencies(expressions, *condition, body_match);
            seek_dependencies(expressions, *then_branch, body_match);
            if let Some(e) = else_branch {
                seek_dependencies(expressions, *e, body_match);
            }
        }
        #[allow(unused)]
        Expr::Loop { body, label } => {
            seek_dependencies(expressions, *body, body_match);
        }
        Expr::Call { callee, args } => {
            seek_dependencies(expressions, *callee, body_match);
            for &arg in args {
                seek_dependencies(expressions, arg, body_match);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            seek_dependencies(expressions, *receiver, body_match);
            for &arg in args {
                seek_dependencies(expressions, arg, body_match);
            }
        }
        Expr::Match {
            expr: match_expr,
            arms,
        } => {
            seek_dependencies(expressions, *match_expr, body_match);
            for arm in arms {
                seek_dependencies(expressions, arm.expr, body_match);
            }
        }
        _ => {}
    }
}
