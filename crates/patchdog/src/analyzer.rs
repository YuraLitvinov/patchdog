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

/// Initializes the Rust Analyzer environment by loading the current project's Cargo workspace and building an analysis database.
/// It discovers the `Cargo.toml` in the current directory, configures `rust-analyzer` to analyze local targets without dependencies, and sets up the `AnalysisHost` with the `RootDatabase` and `Vfs`.
/// Returns an `AnalyzerData` struct containing the initialized database, virtual file system, and local crates, essential for further code analysis operations.
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

/// Gathers contextual code snippets (functions and structs) from the entire Rust project relevant to a specified file and an optional `fn_range`.
/// It iterates through all local crates, modules, and declarations, filtering for items within the target `filepath`.
/// Returns a `HashMap<TextRange, String>` where keys are the `TextRange` of the code item and values are their source code, excluding the target function's own range if `fn_range` is provided.
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
/// Extracts and collects contextual code strings for functions and structs within a given module, specifically when they are located in the target `filepath`.
/// For functions matching `fn_range`, it identifies and retrieves source code for their internal and external dependencies.
/// For structs within the same file, it fetches their entire definition, returning a `HashMap<TextRange, String>` of these code snippets.
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

/// Resolves and returns the `VfsPath` for a given `FunctionId` within the analysis database.
/// It queries the `RootDatabase` to retrieve the function's `FileId` and then uses the `Vfs` to obtain the corresponding file path.
/// Returns `Some(&VfsPath)` if the file ID can be resolved, otherwise `None`.
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

/// Extracts and returns the source code string for a specific `TextRange` within a given file.
/// It parses the file's syntax tree from the `RootDatabase` using its `EditionedFileId`.
/// The function then retrieves the text content of the syntax node or token that covers the specified `TextRange`, providing a string representation of that code segment.
fn print_body(file_id: EditionedFileId, range: TextRange, db: &RootDatabase) -> String {
    let to_source = db.parse(file_id);
    let func = to_source.syntax_node().covering_element(range);
    match func {
        ra_ap_syntax::NodeOrToken::Node(n) => n.text().to_string(),
        ra_ap_syntax::NodeOrToken::Token(t) => t.text().to_string(),
    }
}

/// Retrieves the `TextRange` (span) of a function if that function belongs to a local crate.
/// It queries the `RootDatabase` to get the function's `FileId`, checks if the associated crate is local, and if so, returns the `TextRange` of the function's syntax node.
/// Returns `Some(TextRange)` for local functions or `None` otherwise.
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

/// Analyzes a given `Expr` (expression) within a function's body to identify and collect `FunctionId`s of called functions, and attempts to return their `TextRange`.
/// It utilizes the `hir_def`'s resolver and inference data to find associated function resolutions and method calls.
/// Any discovered `FunctionId`s are added to the provided `fn_ids` `HashSet`, and the `TextRange` of the function is returned if available.
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

/// Recursively traverses the Abstract Syntax Tree (AST) of a function's body, starting from a given `ExprId`, to identify and collect all `ExprId`s that represent expressions.
/// It explicitly delves into complex control flow structures like `if`, `loop`, `call`, `method_call`, and `match` expressions to find nested dependencies.
/// This function mutates the `expressions` `HashSet` in place, adding all discovered expression IDs to build a comprehensive set of the function's operational elements.
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
