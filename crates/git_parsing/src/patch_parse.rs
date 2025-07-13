use git2::{Diff, Patch};
use snafu::{ResultExt, Snafu, OptionExt};
use std::{collections::HashSet, ffi::OsStr, path::Path};
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Git2ErrorHandling {
    #[snafu(display("Unable to read {source}"))]
    Git2Error { source: git2::Error },
    ValueNotPresent,
}
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum HunkChange {
    Add,
    Remove,
    Modify,
}
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct Hunk {
    pub change: HunkChange,
    pub line: usize,
    pub filename: String,
}
impl Hunk {
    pub fn filename(&self) -> String {
        self.filename.to_owned()
    }
    pub fn get_line(&self) -> usize {
        self.line
    }
    pub fn get_change(&self) -> &str {
        match &self.change {
            HunkChange::Add => "Add",
            HunkChange::Remove => "Remove",
            HunkChange::Modify => "Modify",
        }
    }
}
#[derive(Debug)]
pub struct Change {
    pub quantity: usize,
    pub change_at_hunk: Hunk,
}

impl Change {
    pub fn quantity(&self) -> usize {
        self.quantity
    }
    pub fn change_at_hunk(&self) -> Hunk {
        self.change_at_hunk.to_owned()
    }
}
//get_filenames.0 corresponds to old file name, get_filenames.1 corresponds to new file name
//Those can be interchanged, as this only indicates where change occured.
//and may correspond to actual file name change if renaming occurs
pub fn get_filenames(diff: &Diff<'static>) -> Result<Vec<String>, Git2ErrorHandling> {
    let mut vector_of_filenames: Vec<String> = Vec::new();
    for delta in diff.deltas() {
        let new_path = delta
            .new_file()
            .path()
            .map(|p| p.display().to_string())
            .context(ValueNotPresentSnafu)?;
        vector_of_filenames.push(new_path);
    }
    Ok(vector_of_filenames)
}
pub fn git_get_hunks(
    diff: Diff,
    vector_of_filenames: Vec<String>
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut hunk_tuple: Vec<Hunk> = Vec::new();         
    for each_diff in diff.deltas().enumerate() {
        let patch = Patch::from_diff(&diff, each_diff.0).context(Git2Snafu)?;
        for hunk_idx in 0..patch
                                    .as_ref()
                                    .ok_or_else(||
                                        Git2ErrorHandling::Git2Error { 
                                            source: git2::Error::from_str("Invalid operation on line 78")
                                        })?
                                    .num_hunks() 
        {
            let (_hunk, _) = 
                patch
                    .as_ref()
                    .ok_or_else(||
                        Git2ErrorHandling::Git2Error { 
                            source: git2::Error::from_str("Invalid operation on line 86") 
                        })?
                    .hunk(hunk_idx)
                    .context(Git2Snafu)?;
            for line_idx in 0..patch
                .as_ref()
                .ok_or_else(||
                    Git2ErrorHandling::Git2Error { 
                        source: git2::Error::from_str("Invalid operation on line 94") 
                    })?                
                .num_lines_in_hunk(hunk_idx).context(Git2Snafu)?
            {
                let lines = patch
                    .as_ref()
                    .ok_or_else(||
                        Git2ErrorHandling::Git2Error { 
                            source: git2::Error::from_str("Invalid operation on line 75") 
                        })?                    
             .line_in_hunk(hunk_idx, line_idx).context(Git2Snafu)?;

                let line_processed = lines.new_lineno()
                .context(ValueNotPresentSnafu)?
                as usize;
                match lines.origin() {
                    '+' => {
                        hunk_tuple.push(Hunk {
                            change: HunkChange::Add,
                            line: line_processed,
                            filename: vector_of_filenames[each_diff.0].to_owned(),
                        });
                    }
                    '-' => {
                        hunk_tuple.push(Hunk {
                            change: HunkChange::Remove,
                            line: line_processed,
                            filename: vector_of_filenames[each_diff.0].to_owned(),
                        });
                    }
                    ' ' => {
                        hunk_tuple.push(Hunk {
                            change: HunkChange::Modify,
                            line: line_processed,
                            filename: vector_of_filenames[each_diff.0].to_owned(),
                        });
                    }
                    _ => {}
                }
            }
        

    }
}
    Ok(hunk_tuple)

}
pub fn read_non_repeting_functions(
    patch_src: &[u8],
    relative_path: &str,
) -> Result<Vec<String>, Git2ErrorHandling> {
    let mut vec_of_files: Vec<String> = Vec::new();
    let diff = Diff::from_buffer(patch_src).context(Git2Snafu)?;
    let filenames = get_filenames(&diff)?;
    let hunks = git_get_hunks(diff, filenames)?;
    let mut seen = HashSet::new();
    let unique_files = hunks
        .into_iter()
        .filter(|hunk| seen.insert(hunk.filename.to_owned()));
    for list_of_unique_files in unique_files {
        let new_filename = list_of_unique_files.filename();
        let file_extension = Path::new(&new_filename).extension().and_then(OsStr::to_str);
        if let Some("rs") = file_extension {
            let path = format!("{}{}", relative_path, new_filename);
            vec_of_files.push(path);
        }
    }
    Ok(vec_of_files)
}
pub fn remove_repeating(vector_of_objects: Vec<String>) -> Result<Vec<String>, Git2ErrorHandling> {
    let mut vec_of_files: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    let unique_files = vector_of_objects
        .into_iter()
        .filter(|object| seen.insert(object.to_owned()));
    for list_of_unique_files in unique_files {
        let file_extension = Path::new(&list_of_unique_files)
            .extension()
            .and_then(OsStr::to_str);
        if let Some("rs") = file_extension {
            vec_of_files.push(list_of_unique_files);
        }
    }
    Ok(vec_of_files)
}

pub fn match_patch_with_parse(
    relative_path: &str,
    patch_src: &[u8],
) -> Result<Vec<Change>, Git2ErrorHandling> {
    let mut changes: Vec<Change> = Vec::new();
    let list_of_unique_files = read_non_repeting_functions(patch_src, relative_path)?;
    let diff = Diff::from_buffer(patch_src).context(Git2Snafu)?;
    let changed = get_filenames(&diff)?;
    let mut hunks = git_get_hunks(diff, changed)?;
    hunks.sort_by_key(|a| a.filename());
    for each_unique in list_of_unique_files.iter() {
        let mut count = 0;
        for each in &hunks {
            let full_path = relative_path.to_owned() + &each.filename();
            if full_path == *each_unique {
                count += 1;
                match each.change {
                    HunkChange::Add => {
                        changes.push(Change {
                            quantity: count,
                            change_at_hunk: each.to_owned(),
                        });
                    }

                    HunkChange::Remove => {
                        changes.push(Change {
                            quantity: count,
                            change_at_hunk: each.to_owned(),
                        });
                    }
                    HunkChange::Modify => {
                        changes.push(Change {
                            quantity: count,
                            change_at_hunk: each.to_owned(),
                        });
                    }
                };
            }
        }
    }
    println!("Quantity of hunks: {}", hunks.len());
    println!("Quantity of changes: {}", changes.len());
    Ok(changes)
}

pub fn get_easy_hunk(patch_src: &[u8], at_file_path: &str) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut vec_of_hunks: Vec<Hunk> = Vec::new();
    let diff = Diff::from_buffer(patch_src).context(Git2Snafu)?;
    let filenames = get_filenames(&diff)?;
    let hunks = git_get_hunks(diff, filenames)?;
    vec_of_hunks.sort_by_key(|hunk| hunk.filename.to_owned());

    for hunk in hunks {
        if hunk.filename() == at_file_path {
            vec_of_hunks.push(hunk);
        }
    }
    Ok(vec_of_hunks)
}