use git2::{Diff, Patch};
use snafu::{ResultExt, Snafu};
use std::{ffi::OsStr, path::Path};
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Git2ErrorHandling {
    #[snafu(display("Unable to read {source}"))]
    Git2Error { source: git2::Error },
}
//get_filenames.0 corresponds to old file name, get_filenames.1 corresponds to new file name
//Those can be interchanged, as this only indicates where change occured.
//and may correspond to actual file name change if renaming occurs
pub fn get_filenames(
    diff: &Diff<'static>,
) -> Result<Vec<(String, String, usize)>, Git2ErrorHandling> {
    let mut tuple_vector_of_file_names: Vec<(String, String, usize)> = Vec::new();
    let mut i = 0;
    for delta in diff.deltas() {
        i += 1;
        let old_path = delta
            .old_file()
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let new_path = delta
            .new_file()
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        tuple_vector_of_file_names.push((old_path, new_path, i));
    }
    Ok(tuple_vector_of_file_names)
}
pub fn git_get_hunks(
    diff: Diff<'static>,
    tuple_vector_of_file_names: Vec<(String, String, usize)>,
) -> Result<Vec<(&'static str, usize, String)>, Git2ErrorHandling> {
    let mut hunk_tuple: Vec<(&str, usize, String)> = Vec::new();
    for i in diff.deltas().enumerate() {
        let patch = Patch::from_diff(&diff, i.0).context(Git2Snafu)?;

        for hunk_idx in 0..patch.as_ref().unwrap().num_hunks() {
            let (_hunk, _) = patch.as_ref().unwrap().hunk(hunk_idx).unwrap();
            let patch_clone = Patch::from_diff(&diff, i.0).context(Git2Snafu)?;
            for line_idx in 0..patch_clone
                .as_ref()
                .unwrap()
                .num_lines_in_hunk(hunk_idx)
                .unwrap()
            {
                let line = patch_clone
                    .as_ref()
                    .unwrap()
                    .line_in_hunk(hunk_idx, line_idx)
                    .unwrap();
                match line.origin() {
                    '+' => {
                        hunk_tuple.push((
                            "Add",
                            line.new_lineno().unwrap_or(0).try_into().unwrap(),
                            tuple_vector_of_file_names[i.0].1.clone(),
                        ));
                    }
                    '-' => {
                        hunk_tuple.push((
                            "Remove",
                            line.old_lineno().unwrap_or(0).try_into().unwrap(),
                            tuple_vector_of_file_names[i.0].1.clone(),
                        ));
                    }
                    ' ' => {
                        hunk_tuple.push((
                            "Modify",
                            line.old_lineno().unwrap_or(0).try_into().unwrap(),
                            tuple_vector_of_file_names[i.0].1.clone(),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(hunk_tuple)
}

pub fn read_non_repeting_functions(src: &[u8]) -> Result<Vec<String>, Git2ErrorHandling> {
    let mut vec_of_files: Vec<String> = Vec::new();
    let diff = Diff::from_buffer(src).unwrap();
    let filenames = get_filenames(&diff).expect("failed to get filenames");
    let hunks = git_get_hunks(diff, filenames).expect("Unwrap on get_filenames failed");
    if hunks.is_empty() {
        return Ok(vec_of_files);
    }
    let mut last_path = &hunks[0].2;

    for each in hunks.iter().skip(1) {
        if &each.2 != last_path {
            let file_extension = Path::new(last_path)
                .extension()
                .and_then(OsStr::to_str);

            if let Some("rs") = file_extension {
                let path = format!("../../{}", last_path);
                vec_of_files.push(path);
            }

            last_path = &each.2;
        }
    }
    let file_extension = Path::new(last_path)
        .extension()
        .and_then(OsStr::to_str);

    if let Some("rs") = file_extension {
        let path = format!("../../{}", last_path);
        vec_of_files.push(path);
    }

    Ok(vec_of_files)
}