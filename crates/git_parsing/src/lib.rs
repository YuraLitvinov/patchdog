use git2::{Diff, Patch};
use std::fs;
#[derive(Debug)]
pub struct CommitData {
    commit_id: String,
    date: git2::Time,
}

impl CommitData {
    pub fn get_id(&self) -> String {
        self.commit_id.clone()
    }
    pub fn get_date(&self) -> i64 {
        self.date.clone().seconds()
    }
}

pub fn git_get(src: &str) -> Result<(), git2::Error> {
    // Read the patch file into memory
    let patch_text = fs::read(src)
        .expect("Failed to read patch file");

    // Parse the diff from raw patch content
    let diff = Diff::from_buffer(&patch_text)?;

    for (i, delta) in diff.deltas().enumerate() {
        let patch = Patch::from_diff(&diff, i)?;
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

        println!("File: {} -> {}", old_path, new_path);

        for hunk_idx in 0..patch.as_ref().unwrap().num_hunks() {
            let (hunk, _) = patch.as_ref().unwrap().hunk(hunk_idx).unwrap();

            println!(
                "  Hunk starting at old: {}, new: {}",
                hunk.old_start(),
                hunk.new_start()
            );
            let patch_clone = Patch::from_diff(&diff, i)?;
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
                        "    Edited line @ {}: {}",
                        line.old_lineno().unwrap_or(0),
                        String::from_utf8_lossy(line.content()).trim_end()
                    ),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
