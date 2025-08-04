use git2::{Diff, Patch};
use snafu::{OptionExt, ResultExt, Snafu};
use tracing::{event, Level};
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
    fn from (e: git2::Error) -> Self {
       return Git2ErrorHandling::Git2Error { source: e }
    }   
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
}

#[derive(Debug)]
pub struct Change {
    pub quantity: usize,
    pub change_at_hunk: Hunk,
}

/// Matches a patch file with parsed Rust items and generates a vector of `Change` structs.
///
/// # Arguments
///
/// * `relative_path`: The relative path to the file.
/// * `patch_src`: A slice of bytes representing the patch source.
///
/// # Returns
///
/// A `Result` containing a vector of `Change` structs, or a `Git2ErrorHandling` if any error occurred.
pub fn match_patch_with_parse(
    relative_path: &Path,
    patch_src: &[u8],
) -> Result<Vec<Change>, Git2ErrorHandling> {
    let mut changes: Vec<Change> = Vec::new();
    let list_of_unique_files = read_non_repeting_functions(patch_src, relative_path)?;
    let diff = Diff::from_buffer(patch_src)?;
    let changed = get_filenames(&diff)?;
    let mut hunks = git_get_hunks(diff, changed)?;
    hunks.sort_by_key(|a| a.filename());
    for each_unique in list_of_unique_files.iter() {
        let mut count = 0;
        for each in &hunks {
            let full_path = relative_path.join(each.filename());
            if full_path == Path::new(&each_unique) {
                count += 1;
                changes.push(Change {
                    quantity: count,
                    change_at_hunk: each.to_owned(),
                });
            }
        }
    }
    event!(Level::INFO, "Quantity of hunks: {}", hunks.len());
    event!(Level::INFO, "Quantity of changes: {}", changes.len());
    Ok(changes)
}

pub fn get_easy_hunk(patch_src: &[u8], at_file_path: &str) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut vec_of_hunks: Vec<Hunk> = Vec::new();
    let diff = Diff::from_buffer(patch_src)?;
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
/// Extracts hunks from a Git diff, associating them with filenames and line changes.
///
/// # Arguments
///
/// * `diff`: A Git diff object.
/// * `vector_of_filenames`: A vector of filenames.
///
/// # Returns
///
/// A `Result` containing a vector of `Hunk` structs, or a `Git2ErrorHandling` if any error occurred.
fn git_get_hunks(
    diff: Diff<'static>,
    vector_of_filenames: Vec<String>,
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut hunk_tuple: Vec<Hunk> = Vec::new();
    //i returns tuple
    for i in diff.deltas().enumerate() {
        let patch = Patch::from_diff(&diff, i.0)?;
        let patch_ref = patch.as_ref().context(PatchExportSnafu)?;
        for hunk_idx in 0..patch_ref.num_hunks() {
            let (_hunk, _) = patch_ref.hunk(hunk_idx)?;
            for line_idx in 0..patch_ref.num_lines_in_hunk(hunk_idx)? {
                let line = patch_ref
                    .line_in_hunk(hunk_idx, line_idx)?;
                let line_processed: usize = line.new_lineno().unwrap_or(0) as usize;
                let change = match line.origin() {
                    '+' => HunkChange::Add,
                    ' ' => HunkChange::Modify,
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
    patch_src: &[u8],
    relative_path: &Path,
) -> Result<Vec<PathBuf>, Git2ErrorHandling> {
    let mut vec_of_files: Vec<PathBuf> = Vec::new();
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
            let path = relative_path.join(new_filename);
            vec_of_files.push(path);
        }
    }
    Ok(vec_of_files)
}
