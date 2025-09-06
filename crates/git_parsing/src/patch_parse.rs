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
/// Implements the `From` trait to convert a `git2::Error` into a `Git2ErrorHandling::Git2Error` variant.
/// This conversion standardizes the handling of Git-related errors within the application's custom error system.
/// Returns a `Git2ErrorHandling::Git2Error` containing the original `git2::Error`.
    /// Implements the `From` trait to convert a `git2::Error` into a `Git2ErrorHandling::Git2Error`.
    /// This provides a standardized way to integrate `git2` errors into the custom error handling system.
    ///
    /// # Arguments
    ///
    /// * `e` - The `git2::Error` to convert.
    ///
    /// # Returns
    ///
    /// A `Git2ErrorHandling::Git2Error` variant containing the original `git2::Error`.
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

/// Matches parsed Rust code items with hunks from a Git patch.
/// It first identifies unique Rust files involved in the patch, then retrieves all hunks.
/// For each unique file, it finds the corresponding hunk and collects them.
/// Hunks without a matching file are returned as `ChangeType::Remove` which are mitigated later.
///
/// # Arguments
///
/// * `relative_path` - A reference to a `Path` indicating the base directory for relative file paths.
/// * `patch_src` - A reference to a `git2::Diff` object representing the Git patch.
///
/// # Returns
///
/// A `Result<Vec<Hunk>, Git2ErrorHandling>`:
/// - `Ok(Vec<Hunk>)`: A vector of `Hunk` structs, each representing a change in a Rust file that corresponds to a parsed item.
/// - `Err(Git2ErrorHandling)`: If there are issues reading filenames, getting hunks, or other Git2-related errors.
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

/// Extracts hunks from a Git patch that correspond to a specific file path.
/// It first gets all filenames and hunks from the patch, then filters these hunks to include only those belonging to the specified `at_file_path`.
/// The resulting vector of hunks is sorted by filename.
///
/// # Arguments
///
/// * `patch_src` - A reference to a `git2::Diff` object representing the Git patch.
/// * `at_file_path` - A string slice (`&str`) representing the file path for which to retrieve hunks.
///
/// # Returns
///
/// A `Result<Vec<Hunk>, Git2ErrorHandling>`:
/// - `Ok(Vec<Hunk>)`: A vector of `Hunk` structs that are part of the specified file.
/// - `Err(Git2ErrorHandling)`: If there are issues getting filenames or hunks from the patch.
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

/// Extracts the new file paths from a Git `Diff` object.
/// It iterates through each `delta` (change) in the diff and retrieves the path of the `new_file` associated with that change.
/// Returns `Ok(Vec<String>)` containing a vector of file paths (including paths of modified files' new versions) or an `Err(Git2ErrorHandling)` if path extraction fails.
/// Extracts the new file paths from a Git `Diff` object.
/// It iterates through the deltas (changes) in the diff and collects the new file path for each delta.
///
/// # Arguments
///
/// * `diff` - A reference to a `git2::Diff` object.
///
/// # Returns
///
/// A `Result<Vec<String>, Git2ErrorHandling>`:
/// - `Ok(Vec<String>)`: A vector of strings, where each string is the path of a new file or a modified file's new path in the diff.
/// - `Err(Git2ErrorHandling)`: If there is an issue accessing file paths within the diff deltas.
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

/// Processes a Git `Diff` object to extract detailed `Hunk` information, associating changes with specific lines and filenames.
/// It iterates through each delta and its hunks within the diff, determining the `ChangeType` (Add, Modify) for individual lines.
/// Returns `Ok(Vec<Hunk>)` containing a vector of `Hunk` structs, each representing a changed line with its type, line number, and filename, or an `Err(Git2ErrorHandling)` if parsing fails.
fn git_get_hunks(
    diff: &Diff<'static>,
    vector_of_filenames: Vec<String>,
) -> Result<Vec<Hunk>, Git2ErrorHandling> {
    let mut hunk_tuple: Vec<Hunk> = Vec::new();
    //i returns tuple
    for (int, _delta) in diff.deltas().enumerate() {
        let patch = Patch::from_diff(diff, int)?;
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
                    filename: vector_of_filenames[int].to_owned(),
                });
            }
        }
    }
    Ok(hunk_tuple)
}

/// Reads unique Rust file paths from a Git patch, ensuring that only `.rs` files are considered.
/// It first obtains all filenames and hunks from the patch, then filters for unique filenames.
/// For each unique file, it checks if it has a `.rs` extension and constructs its full path relative to `relative_path`.
///
/// # Arguments
///
/// * `patch_src` - A reference to a `git2::Diff` object representing the Git patch.
/// * `relative_path` - A reference to a `Path` indicating the base directory for relative file paths.
///
/// # Returns
///
/// A `Result<Vec<PathBuf>, Git2ErrorHandling>`:
/// - `Ok(Vec<PathBuf>)`: A vector of unique `PathBuf`s corresponding to `.rs` files found in the patch.
/// - `Err(Git2ErrorHandling)`: If there are issues getting filenames or hunks from the patch.
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
