mod tests {
    use crate::binding::find_context;
    use crate::cli::cherrypick_response;
    use gemini::request_preparation::RawResponse;
    use ra_ap_ide_db::RootDatabase;
    use regex::Regex;
    use rust_parsing::object_range::Name;
    use rust_parsing::error::{ErrorBinding, InvalidIoOperationsSnafu};
    use rust_parsing::file_parsing::{FileExtractor, Files, REGEX};
    use salsa::AsDynDatabase;
    use snafu::ResultExt;
    
    use rust_parsing::ErrorHandling;

    use camino::Utf8Path;
    use la_arena::Idx;
    use ra_ap_base_db::{
        BuiltCrateData, Crate, CrateBuilder, CrateWorkspaceData, RootQueryDb, SourceDatabase, SourceRoot, SourceRootId, SourceRootInput
    };
    use rust_parsing::{ObjectRange, rust_parser::{RustItemParser, RustParser}};
    use ra_ap_project_model::{CargoConfig, ProjectManifest, ProjectWorkspace};
    use ra_ap_syntax::ast;
    use ra_ap_syntax::ast::AstNode;
    use ra_ap_syntax::ast::HasName;
    use ra_ap_vfs::{file_set::FileSetConfigBuilder};
    use ra_ap_vfs::{AbsPath, Vfs, VfsPath};
    use rustc_hash::FxHashMap;
    use ra_ap_hir::db::DefDatabase;
    use salsa::Database;
    use std::{collections::HashMap, env, fs, path::{Path, PathBuf}};

    const PATH_BASE: &str = "../../tests/data.rs";


    trait Upcast<T: ?Sized> {
        fn upcast(&self) -> &T;
    }

    impl Upcast<dyn DefDatabase> for RootDatabase {
        fn upcast(&self) -> &(dyn DefDatabase + 'static) {
            &*self
        }
    }
    impl Upcast<dyn SourceDatabase> for RootDatabase {
        fn upcast(&self) -> &dyn SourceDatabase {
            self
        }
    }
    impl Upcast<dyn RootQueryDb> for RootDatabase {
        fn upcast(&self) -> &dyn RootQueryDb {
            self
        }
    }

    #[test]
    fn analyzer() -> Result<(), ErrorHandling> {
        tracing_subscriber::fmt::init();

        let absolute = env::current_dir()?;
        let mut db = ra_ap_ide_db::RootDatabase::default();
        let mut def = Vfs::default();
        let mut f_id = |x: &AbsPath| {
            let path = VfsPath::new_real_path(x.to_string());
            def.set_file_contents(path.clone(), fs::read(path.to_string()).ok());
            if let Some(map) = def.file_id(&path) {
                Some(map.0)
            } else {
                None
            }
        };
        let as_absolute = absolute.join("../../Cargo.toml");
        let utf8path = AbsPath::assert(Utf8Path::new(as_absolute.to_str().unwrap()));
        let manifest = ProjectManifest::discover_single(utf8path).unwrap();
        let cargo_config = CargoConfig::default();
        let progress = |msg: String| println!("progress: {msg}");
        let ws = ProjectWorkspace::load(manifest, &cargo_config, &progress).unwrap();
        let mut fx = FxHashMap::default();
        let graph = ws.to_crate_graph(&mut f_id, &mut fx).0;
        graph.clone().set_in_db(&mut db);
        let idxs = graph.iter().map(|f| f).collect::<Vec<Idx<CrateBuilder>>>();
        for each in idxs.clone() {
            let each = graph[each].clone();
                let f = def.file_path(each.basic.root_file_id);
                let text = fs::read_to_string(f.to_string())
                    .expect("Couldn't read file to string");
                db.set_file_text_with_durability(
                    each.basic.root_file_id,
                    &text,
                    salsa::Durability::LOW,
                );
            
        }

        let ws_data = triomphe::Arc::new(CrateWorkspaceData {
            data_layout: ws.target_layout.clone(),
            toolchain: ws.toolchain.clone(),
        });
        let cratedb: &dyn SourceDatabase = db.upcast();
        let crates = idxs
            .clone()
            .into_iter()
            .map(|each| {
                let each = graph[each].clone();
                    let data = BuiltCrateData {
                        root_file_id: each.basic.root_file_id,
                        edition: each.basic.edition,
                        dependencies: vec![],
                        origin: each.basic.origin,
                        is_proc_macro: each.basic.is_proc_macro,
                        proc_macro_cwd: each.basic.proc_macro_cwd,
                    };
                    let return_crate = Crate::builder(
                        data,
                        each.extra,
                        ws_data.clone(),
                        each.cfg_options,
                        each.env,
                    )
                    .new(cratedb);
                    return_crate
            })
            .collect::<Vec<Crate>>();
        /*for each in crates.into_iter().enumerate() {
            let krate = each.1;
            let crate_root = Path::new(&krate.env(&db).get("CARGO_MANIFEST_DIR").unwrap()).join("src");
            let path = def.file_path(krate.data(&db).root_file_id);
            let mut modules = HashMap::new();
            get_path(Path::new(&path.to_string()), Path::new(&crate_root),"", &mut modules)?;
            let roots = modules.into_iter().map(|f| 
                VfsPath::new_real_path(f.1.to_str().unwrap().to_string())
            ).collect::<Vec<VfsPath>>();
            let mut builder = FileSetConfigBuilder::default();
            builder.add_file_set(roots.clone());
            let config = builder.build(); 
            let sets = config.partition(&def)[1].to_owned();
            let new_local = triomphe::Arc::new(SourceRoot::new_local(sets.clone()));
            db.set_source_root_with_durability(
                SourceRootId(each.0 as u32),
                new_local,
                salsa::Durability::LOW
            );
            db.source_root_crates(SourceRootId(each.0 as u32));
        }
        */
        println!("{}", "test");
        let d = db.all_crates();
        println!("{:#?}", d);

        assert_eq!(true, false);
        Ok(())
    }


    fn get_path(
    file: &Path,                  // current file (e.g. src/lib.rs)
    root_dir: &Path,              // crate root dir (e.g. src)
    prefix: &str,                 // module path prefix ("", or "foo::bar")
    modules: &mut HashMap<String, PathBuf>,
) -> Result<(), ErrorHandling> {
    let text = fs::read_to_string(file)?;
    let parsed = RustItemParser::parse_all_rust_items(&text)?
        .into_iter()
        .filter(|parse| parse.names.type_name == "mod")
        .collect::<Vec<ObjectRange>>();

    for item in parsed {
        let mod_name = &item.names.name; // assume parser extracts `foo` from `mod foo;`
        let fq_name = if prefix.is_empty() {
            mod_name.clone()
        } else {
            format!("{prefix}::{mod_name}")
        };

        // Candidate paths for this module
        let candidate1 = root_dir.join(format!("{mod_name}.rs"));
        let candidate2 = root_dir.join(mod_name).join("mod.rs");

        let path = if candidate1.exists() {
            candidate1
        } else if candidate2.exists() {
            candidate2
        } else {
            return Err(ErrorHandling::CouldNotGetLine);
        };

        modules.insert(fq_name.clone(), path.clone());

        // Recurse into that module file
        get_path(&path, path.parent().unwrap_or(root_dir), &fq_name, modules)?;
    }

    Ok(())
}

    fn _handle_item(item: ast::Item) {
        match item {
            ast::Item::AsmExpr(i) => {
                println!("Found asm expr: {:?}", i.syntax().text());
            }
            ast::Item::Const(i) => {
                println!("Found const: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Enum(i) => {
                println!("Found enum: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::ExternBlock(i) => {
                println!("Found extern block: {:?}", i.syntax().text());
            }
            ast::Item::ExternCrate(i) => {
                println!("Found extern crate: {:?}", i.syntax().text());
            }
            ast::Item::Fn(i) => {
                println!("Found fn: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Impl(i) => {
                println!("Found impl: {:?}", i.syntax().text());
            }
            ast::Item::MacroCall(i) => {
                println!("Found macro call: {:?}", i.syntax().text());
            }
            ast::Item::MacroDef(i) => {
                println!("Found macro def: {:?}", i.syntax().text());
            }
            ast::Item::MacroRules(i) => {
                println!("Found macro rules: {:?}", i.syntax().text());
            }
            ast::Item::Module(i) => {
                println!("Found module: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Static(i) => {
                println!("Found static: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Struct(i) => {
                println!("Found struct: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Trait(i) => {
                println!("Found trait: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::TraitAlias(i) => {
                println!(
                    "Found trait alias: {:?}",
                    i.name().map(|n| n.text().to_owned())
                );
            }
            ast::Item::TypeAlias(i) => {
                println!(
                    "Found type alias: {:?}",
                    i.name().map(|n| n.text().to_owned())
                );
            }
            ast::Item::Union(i) => {
                println!("Found union: {:?}", i.name().map(|n| n.text().to_owned()));
            }
            ast::Item::Use(i) => {
                println!("Found use: {:?}", i.use_token().unwrap().text_range());
            }
        }
    }
    
    /// Tests the `parse_all_rust_items` function by reading a Rust file from `PATH_BASE` and parsing its contents.
    /// It then iterates through the parsed objects, printing `impl` objects.
    /// The test asserts `true` for `true`, which is a placeholder and does not validate the parsing logic itself.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The file at `PATH_BASE` cannot be read.
    /// - `parse_all_rust_items` fails to parse the source.
        #[test]
        fn test_parse() {
            let source = fs::read_to_string(PATH_BASE).expect("File read failed");
            let parsed = RustItemParser::parse_all_rust_items(&source).expect("Parsing failed");
            for object in parsed {
                let obj_type = object.names.type_name.clone();
                if obj_type == "impl".to_string() {
                    println!("{:?}", object);
                }
            }

            assert_eq!(true, true);
        }

    /// Tests the `parse_all_rust_items` function to find all function definitions in a Rust file.
    /// It reads a Rust file from `PATH_BASE`, parses its contents, and then iterates through the parsed objects.
    /// If an object is identified as a function (`"fn"`), its debug representation is printed.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The file at `PATH_BASE` cannot be read.
    /// - `parse_all_rust_items` fails to parse the source.
        #[test]
        fn find_all_fn() {
            let source = fs::read_to_string(PATH_BASE)
                .context(InvalidIoOperationsSnafu { path: PATH_BASE })
                .expect("Failed to read file");
            let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
            for object in parsed {
                let obj_type = object.names.type_name.clone();
                if obj_type == "fn".to_string() {
                    println!("{:?}", object);
                }
            }
        }

    /// Tests the `find_module_file` function to ensure it correctly locates module files.
    /// It reads a test Rust file (`../../tests/lib.rs`), parses it to find module declarations.
    /// For each module found, it attempts to locate its corresponding file using `find_module_file`.
    /// The collected module file paths are then joined into a single string and compared against an `expected_behavior` string.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The test file cannot be read.
    /// - Parsing of the test file fails.
    /// - `find_module_file` fails to locate a declared module file.
    /// - The assertion `assert_eq!` fails, indicating a mismatch between expected and actual module file paths.
        #[test]
        fn test_find_module_files() {
            let expected_behavior: &str = "../../tests/test_lib.rs\n../../tests/data.rs";
            let path = Path::new("../../tests/lib.rs");
            let source = fs::read_to_string(&path)
                .context(InvalidIoOperationsSnafu { path: path })
                .expect("Failed to read file");
            let parsed = RustItemParser::parse_all_rust_items(&source).expect("Failed to parse");
            let mut obj_vector: Vec<String> = Vec::new();
            for object in parsed {
                let obj_type = object.names.type_name;
                let obj_name = object.names.name;
                if obj_type == "mod".to_string() {
                    let module_location =
                        RustItemParser::find_module_file(path.to_path_buf(), obj_name.to_owned())
                            .expect("Couldn't find mod file");
                    obj_vector.push(module_location.unwrap().to_string_lossy().to_string());
                }
            }

            assert_eq!(expected_behavior, obj_vector.join("\n"));
        }

    /// Tests the ability to read and construct file paths for arguments.
    /// It retrieves the current working directory, manipulates it to construct a mock `path_to_patch`,
    /// and then asserts `true` for `true`, which is a placeholder and does not perform active path validation.
    /// The commented-out `assert_eq!` suggests an original intention to validate the constructed path.
    ///
    /// # Panics
    ///
    /// This test will panic if `env::current_dir()` fails to retrieve the current directory.
        #[test]
        fn test_read_argument() {
            let mut path = env::current_dir().expect("couldn't get path");
            path.pop();
            path.pop();
            let _path_to_patch = path.join("patch.patch");
            /*
            assert_eq!(
                path_to_patch,
                Path::new("/home/runner/work/patchdog/patchdog/patch.patch")
            );
            */
            assert_eq!(true, true);
        }

    /// A placeholder test function intended for covering scenarios with empty objects.
    /// The commented-out code suggests an initial design involving `Name`, `LineRange`, and `ObjectRange` structs,
    /// but no actual test logic is implemented.
    ///
    /// This test currently does nothing.
        #[test]
        fn test_cover_empty_object() {
            /*
            let mut name: Vec<Name> = Vec::new();
            let mut ranges: Vec<LineRange> = Vec::new();
            let mut _objectrange: Vec<ObjectRange> = Vec::new();
            */
        }

    /// Tests the `parse_all_rust_items` function's ability to identify and parse comments within a Rust file.
    /// It reads a specific Rust file (`../../crates/patchdog/src/binding.rs`), parses its content,
    /// and then prints the debug representation of each parsed object, including comments.
    /// The comment `//block is of 94 symbols length` appears to be a note about expected content for testing.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The specified file cannot be read.
    /// - `parse_all_rust_items` fails to parse the file content.
        #[test]
        fn find_comments() {
            //block is of 94 symbols length
            let file = fs::read_to_string("../../crates/patchdog/src/binding.rs").expect("err on 159");
            let parsed = RustItemParser::parse_all_rust_items(&file).expect("err");
            for each in parsed {
                println!("{:?}", each);
            }
        }
        const _JSON: &str = r#"{
  "files": [
    {
      "filename": "/home/yurii-sama/patchdog/crates/gemini/src/lib.rs",
      "types": {
        "fn": [
          {
            "name": "req_res",
            "comment": "/// Sends the given file content to the Gemini model and returns the generated response.\n/// \n/// # Arguments\n/// \n/// * `file_content` - A string representation of the file to send to the model.\n/// \n/// # Returns\n/// \n/// A `Result` containing the model's response as a `String`, or an error if the request fails."
          }
        ],
        "impl": [
          {
            "name": "GoogleGemini",
            "comment": "/// Implementation of the Gemini API interaction logic."
          }
        ],
        "const": [],
        "struct": [],
        "enum": [],
        "trait": [],
        "type": []
      }
}]}"#;

    /// A placeholder test function for validating the output from an AI agent.
    /// The commented-out code outlines a plan to:
    /// 1. Assess if the AI agent's JSON output is valid.
    /// 2. Implement recursive calls to retry if the output is invalid or missing required fields (filename, function name).
    /// The test currently returns `Ok(())` without executing any of the assessment logic.
    ///
    /// This test currently does nothing.
        #[test]
        fn test_agent_out() -> Result<(), ErrorBinding> {
            /*
            1. We need to assess whether the JSON given by the AI Agent is valid first-hand. If it's not, then we recursively call
            function to run again and again until there is a proper response.
            serde_json(json) is_err(): retry
            2. Filename is_err(): retry
            3. search function is.err(): retry
            //call_agent - using it as mock for calling AI Agent again

            let assessed = assess_correct_output(JSON.to_string())?;
            let expected = Assess {
                filename: "/home/yurii-sama/patchdog/crates/gemini/src/lib.rs".to_string(),
                names: vec!["req_res".to_string()],
            };
            assert_eq!(assessed[0], expected);
            */
            Ok(())
        }

    /// Tests the `cherrypick_response` function's ability to extract specific JSON objects using a regex.
    /// It reads a test JSON file (`../../tests/response_regex.json`), applies `cherrypick_response` to it,
    /// and asserts that the number of extracted `RawResponse` objects is `3`.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The test JSON file cannot be read.
    /// - `cherrypick_response` returns an error.
    /// - The assertion `assert_eq!` fails, meaning the number of extracted responses is not `3`.
        #[test]
        fn test_regex() {
            let test = fs::read_to_string(Path::new("../../tests/response_regex.json")).unwrap();
            let assess_size = cherrypick_response(&test).unwrap();
            assert_eq!(assess_size.len(), 3);
        }

    /// Tests various parsing scenarios for agent responses, including handling of malformed JSON.
    /// It uses a predefined regex (`REGEX`) to cherry-pick responses and also attempts direct JSON deserialization.
    /// If direct deserialization fails (as expected for certain test cases), it performs a fallback mechanism by manipulating the response string (removing first and last lines, joining) and re-attempts deserialization.
    /// Assertions check the lengths of the parsed results from both methods.
    ///
    /// # Returns
    ///
    /// An `Ok(())` on successful completion of the tests.
    /// An `Err(ErrorHandling)` if any file operation or parsing (including regex or JSON) fails unexpectedly.
        #[test]
        fn test_response() -> Result<(), ErrorHandling> {
            let re = Regex::new(REGEX).unwrap();
            let test = fs::read_to_string(Path::new("../../tests/response_regex.json")).unwrap();
            let mut assess_size = vec![];
            for cap in re.captures_iter(&test) {
                let a = cap.get(0).unwrap().as_str();
                let to_struct = serde_json::from_str::<RawResponse>(a).unwrap();
                assess_size.push(to_struct);
            }
            match serde_json::from_str::<Vec<RawResponse>>(&test) {
                Ok(ok) => {
                    println!("{} = {}", ok.len(), assess_size.len());
                }
                Err(_) => {
                    let as_vec = FileExtractor::string_to_vector(&test);
                    let a = &as_vec[1..as_vec.len() - 1].join("\n");
                    let to_struct = serde_json::from_str::<Vec<RawResponse>>(a).unwrap();
                    assert_eq!(assess_size.len(), 3);
                    assert_eq!(to_struct.len(), 58);
                }
            }
            Ok(())
        }

    /// Tests advanced parsing and matching capabilities of Rust code, specifically focusing on function bodies and expressions.
    /// It reads a Rust file (`../../crates/patchdog/src/binding.rs`), parses all its items,
    /// filters for functions, and then for each function, it attempts to parse its body using `syn`.
    /// The test includes extensive pattern matching to explore various `Stmt` and `Expr` types within the AST, printing specific identifiers or debug information.
    /// This test primarily serves as a diagnostic tool for AST traversal and does not contain explicit assertions for correctness beyond `unwrap()`.
    ///
    /// # Panics
    ///
    /// This test will panic if:
    /// - The specified file cannot be read.
    /// - `parse_all_rust_items` fails to parse the file content.
    /// - `syn::parse_file` fails for any function body.
    /// - Any `unwrap()` call fails during AST traversal.


        #[test]
        fn test_parsing_matching() {
            let path = env::current_dir().unwrap().join("src/cli.rs");
            let file = fs::read_to_string(&path).unwrap();
            let as_vec = &FileExtractor::string_to_vector(&file)[90..131].join("\n");
            let _ = find_context(path, "test", as_vec).unwrap();
        }
        #[test]
        fn test_match() {

            let names = Name {
                type_name: "fn".to_string(),
                name: "request_manager".to_string()
            };
            let _input =  ObjectRange {
                line_ranges: 395..429,
                names
            };

            //match_context(context: Vec<(PathBuf, ObjectRange)>) -> HashMap<String, PathObject>
            //let deserial_in = serde_json::from_str::<ObjectRange>(input).expect("err failed to parse from json ObjectRange");
            //let deserial_out = serde_json::from_str::<PathObject>(output).expect("err failed to parse from json PathObject");
        }
    
}
