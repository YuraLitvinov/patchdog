use git2::{Diff, Patch};
use rayon::prelude::*;
use snafu::{OptionExt, Snafu};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Git2ErrorHandling {
    #[snafu(display("Unable to read {source}"))]
    Git2Error {
        source: git2::Error,
    },
    PatchExportError,
}
impl From<git2::Error> for Git2ErrorHandling {
    fn from(e: git2::Error) -> Self {
        Git2ErrorHandling::Git2Error { source: e }
    }
}
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum ChangeType {
    Add,
    Remove,
    Modify,
}
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct Hunk {
    pub change: ChangeType,
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
}

pub fn match_patch_with_parse(
    relative_path: &Path,
    patch_src: &Diff<'static>,
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let list_of_unique_files = read_non_repeting_functions(patch_src, relative_path)?;
    let changed = get_filenames(patch_src)?;
    let mut hunks = git_get_hunks(patch_src, changed)?;
    hunks.sort_by_key(|a| a.filename());
    let changes: Vec<Hunk> = list_of_unique_files
        .par_iter()
        .map(|each_unique| {
            let collected = hunks.par_iter().find_first(|each| {
                let full_path = relative_path.join(each.filename());
                full_path == Path::new(&each_unique)
            });
            if let Some(collected) = collected {
                collected.to_owned()
            } else {
                //Here returning Remove, as it is mitigated afterwards
                Hunk {
                    change: ChangeType::Remove,
                    line: 0,
                    filename: String::new(),
                }
            }
        })
        .collect::<Vec<Hunk>>();
    Ok(changes)
}

pub fn get_easy_hunk(
    patch_src: &Diff<'static>,
    at_file_path: &str,
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut vec_of_hunks: Vec<Hunk> = Vec::new();
    let filenames = get_filenames(patch_src)?;
    let hunks = git_get_hunks(patch_src, filenames)?;
    vec_of_hunks.sort_by_key(|hunk| hunk.filename.to_owned());

    for hunk in hunks {
        if hunk.filename() == at_file_path {
            vec_of_hunks.push(hunk);
        }
    }
    Ok(vec_of_hunks)
}

fn get_filenames(diff: &Diff<'static>) -> Result<Vec<String>, Git2ErrorHandling> {
    let mut vector_of_filenames: Vec<String> = Vec::new();
    for delta in diff.deltas() {
        let new_path = delta
            .new_file()
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        vector_of_filenames.push(new_path);
    }
    Ok(vector_of_filenames)
}

fn git_get_hunks(
    diff: &Diff<'static>,
    vector_of_filenames: Vec<String>,
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut hunk_tuple: Vec<Hunk> = Vec::new();
    //i returns tuple
    for i in diff.deltas().enumerate() {
        let patch = Patch::from_diff(diff, i.0)?;
        let patch_ref = patch.as_ref().context(PatchExportSnafu)?;
        for hunk_idx in 0..patch_ref.num_hunks() {
            let (_hunk, _) = patch_ref.hunk(hunk_idx)?;
            for line_idx in 0..patch_ref.num_lines_in_hunk(hunk_idx)? {
                let line = patch_ref.line_in_hunk(hunk_idx, line_idx)?;
                let line_processed = line.new_lineno().unwrap_or(0) as usize;
                let change = match line.origin() {
                    '+' => ChangeType::Add,
                    ' ' => ChangeType::Modify,
                    _ => continue,
                };
                hunk_tuple.push(Hunk {
                    change,
                    line: line_processed,
                    filename: vector_of_filenames[i.0].to_owned(),
                });
            }
        }
    }
    Ok(hunk_tuple)
}

fn read_non_repeting_functions(
    patch_src: &Diff<'static>,
    relative_path: &Path,
) -> Result<Vec<PathBuf>, Git2ErrorHandling> {
    let mut vec_of_files: Vec<PathBuf> = Vec::new();
    let filenames = get_filenames(patch_src)?;
    let hunks = git_get_hunks(patch_src, filenames)?;
    let mut seen = HashSet::new();
    let unique_files = hunks
        .into_iter()
        .filter(|hunk| seen.insert(hunk.filename.to_owned()));
    for list_of_unique_files in unique_files {
        let new_filename = list_of_unique_files.filename();
        let file_extension = Path::new(&new_filename).extension().and_then(OsStr::to_str);
        if let Some("rs") = file_extension {
            let path = relative_path.join(new_filename);
            vec_of_files.push(path);
        }
    }
    Ok(vec_of_files)
}
