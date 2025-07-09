use git2::{Diff, Patch};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Git2ErrorHandling {
    #[snafu(display("Unable to read {source}"))]
    Git2Error {
        source: git2::Error,
    },
}
//get_filenames.0 corresponds to old file name, get_filenames.1 corresponds to new file name
//Those can be interchanged, as this only indicates where change occured. 
//and may correspond to actual file name change if renaming occurs
pub fn get_filenames(diff: Diff<'static>) -> Result<Vec<(String, String)>, Git2ErrorHandling> {
    let mut tuple_vector_of_file_names: Vec<(String, String)> = Vec::new();
    for delta in diff.deltas() {
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
        tuple_vector_of_file_names.push((old_path, new_path));
    }
    Ok(tuple_vector_of_file_names)
}
pub fn git_get_hunks(diff: Diff<'static>) -> Result<(), Git2ErrorHandling> {
    for i in diff.deltas().enumerate() {
        let patch = Patch::from_diff(&diff, i.0).context(Git2Snafu)?;

        for hunk_idx in 0..patch.as_ref().unwrap().num_hunks() {
            let (hunk, _) = patch.as_ref().unwrap().hunk(hunk_idx).unwrap();

            println!(
                "  Hunk starting at old: {}, new: {}",
                &hunk.old_start(),
                &hunk.new_start()
            );
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
                    '+' => println!(
                        "    Added   line @ {}: {}",
                        line.new_lineno().unwrap_or(0),
                        String::from_utf8_lossy(line.content()).trim_end()
                    ),
                    '-' => println!(
                        "    Removed line @ {}: {}",
                        line.old_lineno().unwrap_or(0),
                        String::from_utf8_lossy(line.content()).trim_end()
                    ),
                    ' ' => println!(
                        "    Edited line @ {}: ",
                        line.old_lineno().unwrap_or(0),
                        //String::from_utf8_lossy(line.content()).trim_end()
                    ),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
