use syn::spanned::Spanned;
use syn::{
    parse_file, parse_quote, Attribute, File, Item
};
use quote::quote;
//use std::io::prelude::*;
use std::io::Result as Result;
use std::fs;
use std::path::{Path, PathBuf};
use ra_ap_syntax::{SourceFile, SyntaxNode};
use std::io::Write;
#[derive(Debug)]
enum LineRange {
    Start(usize),
    End(usize),
}
#[derive(Debug)]
enum Names {
    TypeName(&'static str),
    Name(String),
}
#[derive(Debug)]
pub struct ObjectRange {
    line_range: Vec<LineRange>,
    name: Vec<Names>
    
}
/*
Calling each object with ObjectRange 
object_name = %object%.object_name().unwrap()
object_type = %object%.object_type().unwrap()
line_start = %object%.line_start().unwrap()
line_end = %object%.line_end().unwrap()
*/
impl ObjectRange {
    pub fn object_name(&self) -> Option<String> {
        for n in &self.name {
            if let Names::Name(val) = n {
                return Some(val.to_string());
            }
        }
        None
    }
    pub fn object_type(&self) -> Option<String> {
        for n in &self.name {
            if let Names::TypeName(val) = n {
                return Some(val.to_string());
            }
        }
        None
    }
    pub fn line_start(&self) -> Option<usize> {
        for r in &self.line_range {
            if let LineRange::Start(val) = r {
                return Some(*val);
            }
        }
        None
    }
    pub fn line_end(&self) -> Option<usize> {
        for r in &self.line_range {
            if let LineRange::End(val) = r {
                return Some(*val);
            }
        }
        None
    }
}

pub fn frontend_visit_items(item: &ObjectRange) -> Vec<&ObjectRange> {
    vec![item]
}

pub fn parse(file_content: String) -> Vec<String> {
    let ast: File = parse_file(&file_content).expect("Unable to parse file");
    let ast: Vec<String> = ast.items.into_iter().filter_map(|item| {
        match item {
            Item::Fn(func) => Some(format!("Function: {}", func.sig.ident)),
            Item::Struct(s) => Some(format!("Struct: {}", s.ident)),
            _ => None,
        
        }
    }).collect();
    ast
}

pub fn write_to_file(response: String, name: String) -> Result<()> {
    let file_name = name + "DOC";
    let replace = response.replace("```rust", "");
    let res = replace.replace("```", "");
    println!("{}", file_name);
    let mut file = std::fs::File::create(file_name)?;
    file.write_all(res.as_bytes())?;
    Ok(())
}

pub fn file_deserialize(file_path: &str) -> Result<Vec<String>> {
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);
    let paths: Vec<String> = serde_json::from_reader(reader)?;
    Ok(paths)
}

pub fn my_parse_file(file_path: &str) -> String {
    let src = std::fs::read_to_string(file_path).expect("unable to read file");
    let syntax = parse_file(&src).expect("unable to parse file");
    let syntax_tree = format!("{:#?}", &syntax);
    //let output = write_to_file(syntax_tree.to_string(), "Tree_of_file".to_string());
    syntax_tree 
    //println!("{:?}", syntax_tree);
    //Ok(())
}

pub fn transform(ast: &str) {
    let mut ast = parse_file(ast).expect("Unable to parse file");
    for item in &mut ast.items {
        if let Item::Fn(func) = item {
            let doc_attr: Attribute = parse_quote!(#[doc = "Modified"]);
            func.attrs.push(doc_attr);
        }
    }
    // Turn back to code
    let tokens = quote!(#ast);
    println!("{}", tokens);
    let _ =write_to_file(tokens.to_string(), "123".to_string());
}


pub fn parse_all_rust_items(path: &Path) -> Vec<ObjectRange> { //Depends on visit_items and find_module_file
    let src = fs::read_to_string(path).expect("Could not read");
    //println!("{:?}", &path);
    let ast: File = parse_file(&src).expect("Could not parse");
    visit_items(&ast.items)
} 


fn visit_items(items: &[Item]) -> Vec<ObjectRange> {
    let mut object_line: Vec<ObjectRange> = Vec::new();
    for item in items {
        match item {
            Item::Struct(s) => {  
                let line_start = s.span().start().line;
                let line_end = s.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("struct"), Names::Name(s.ident.to_string())],
                });
            },
            Item::Enum(e) => { 
                let line_start = e.span().start().line;
                let line_end = e.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)], 
                    name: vec![Names::TypeName("enum"), Names::Name(e.ident.to_string())],
                });

            },
            Item::Fn(f) => { 
                let line_start = f.span().start().line;
                let line_end = f.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("fn"), Names::Name(f.sig.ident.to_string())],
                });
            },
            
            Item::Mod(m) => {     
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(m.span().start().line), LineRange::End(m.span().end().line)],
                    name: vec![Names::TypeName("mod"), Names::Name(m.ident.to_string())],
                });
                /* 
                if let Some((_, items)) = &m.content {
                    // Inline module
                    visit_items(items, base_path);
                } else {
                    // External module: look for file on disk
                    let mod_path = find_module_file(base_path, &m.ident.to_string());
                    if let Some(mod_file) = mod_path {
                        parse_all_rust_items(&mod_file);
                    }
                }
                */
            },
             
            Item::Use(u) => {   
                if let syn::UseTree::Path(path) = u.tree.to_owned() {
                    let path_name = path.ident.to_string();
                    let start =path.span().start().line;
                    let end = path.span().end().line;
                    object_line.push(ObjectRange {
                        line_range: vec![LineRange::Start(start), LineRange::End(end)],
                        name: vec![Names::TypeName("use"), Names::Name(path_name)],
                    });
                    
                }   
            
            
            },

            Item::Impl(i) => {
                let line_start = i.span().start().line;
                let line_end= i.span().end().line;
                //let impl_name = format!("{:?}", i.trait_.as_ref().unwrap().1.get_ident().expect("REASON").to_string());
                let trait_name = match &i.trait_ {
                    Some(trait_) => trait_.1.get_ident().expect("REASON").to_string(),
                    None => "matches struct".to_string(),
                };
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("impl"), Names::Name(trait_name)],
                });
            },
            Item::Trait(t) => { 
               let line_start = t.span().start().line;
               let line_end = t.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("trait"), Names::Name(t.ident.to_string())],
                });
                                }, 
            Item::Type(t) => { 
                let line_start = t.span().start().line;
                let line_end= t.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("type"), Names::Name(t.ident.to_string())],
                });
            },
            Item::Union(u) => { 
                let line_start = u.span().start().line;
                let line_end= u.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("union"), Names::Name(u.ident.to_string())],
                });
            },
            Item::Const(c) => { 
                let line_start = c.span().start().line;
                let line_end= c.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("const"), Names::Name(c.ident.to_string())],
                });
            },            
            Item::Macro(m) => { 
                let macro_name = format!("{:?}", m.ident.clone().unwrap());
                let line_start = m.span().start().line;
                let line_end= m.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("macro"), Names::Name(macro_name)],
                });
            },           
        
            Item::ExternCrate(c) => { 
                let line_start = c.span().start().line;
                let line_end= c.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("extern crate"), Names::Name(c.ident.to_string())],
            });
        },

            Item::Static(s) => {  
                let line_start = s.span().start().line;
                let line_end = s.span().end().line;
                object_line.push(ObjectRange {
                    line_range: vec![LineRange::Start(line_start), LineRange::End(line_end)],
                    name: vec![Names::TypeName("static"), Names::Name(s.ident.to_string())],
                });
            },        
            _ => println!("Other item"),

        }
        
    }
    object_line
}

pub fn find_module_file(base_path: &Path, mod_name: &str) -> Option<PathBuf> {
    let paths = [
        base_path.join(format!("{}.rs", mod_name)),               // mod.rs style
        base_path.join(mod_name).join("mod.rs"),                  // mod.rs in subdirectory
    ];

    for path in &paths {
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    None
}

pub fn analyzer(source: &str) -> String {
    let parse = SourceFile::parse(source, ra_ap_syntax::Edition::Edition2024);
    let syntax_node: SyntaxNode = parse.syntax_node();
    // Format the syntax tree as a string
    let ast_string = format!("{:#?}", syntax_node);
    ast_string


}

pub fn file_to_vector(file: &Path) -> Vec<String> { //Simplified version, using the standard library; functions virtually the same
    let code = fs::read_to_string(file).expect("Failed to read file");
    code.lines().map(|line| line.to_string()).collect()
}

pub fn extract_function(from: &Path, line_start: &usize, line_end: &usize) -> String { 
    let vector_of_file = file_to_vector(from);
    let line_start = line_start - 1;
    let f = &vector_of_file[line_start..*line_end].join("\n");
    //parse_all_rust_items(std::path::Path::new(f));
    //println!("{}", f);
    f.to_string()
}

pub fn find_object(line_start: usize, line_end: usize, code_block: Vec<String>){ //Determines whether the provided line range belongs to a function
    let line_start: usize = line_start - 1;
    let f = &code_block[line_start..line_end].join("\n");
    
    println!("{}", f);

}


