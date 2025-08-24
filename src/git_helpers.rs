use git2::{DiffFormat, DiffOptions, Repository};
use std::{collections::HashMap, path::PathBuf, str};
use crate::config::DiffProfile;

/// Structure that allow to contain both the diff and the edited source file for commits or for staged edits
pub struct ExpandedCommit {
    pub diffs: Option<Vec<String>>,
    pub sources: Option<Vec<PathBuf>>,
}
impl ExpandedCommit {
    pub fn new() -> ExpandedCommit {
        ExpandedCommit {
            diffs: None,
            sources: None,
        }
    }
}

/// Get an ExpandedCommit rappresenting staged edits 
pub fn staged_diffs(diff_profile: DiffProfile) -> Result<ExpandedCommit, git2::Error> {
    let repo = Repository::open(".")?;
    let index = repo.index()?;

    // Try to get HEAD tree. If repo has no commits yet, treat HEAD tree as None.
    let head_tree = match repo.head() {
        Ok(reference) => Some(reference.peel_to_tree()?),
        Err(_) => None,
    };

    let mut diff_opts = DiffOptions::new();
    // Customize diff_opts if you want (context lines, pathspecs, etc.)
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), Some(&mut diff_opts))?;

    // Map path -> patch text
    let mut file_patches: HashMap<PathBuf, String> = HashMap::new();

    // Print the diff in patch format; the closure is called for every diff line.
    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        // Prefer new file path; fall back to old file path.
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("unknown"));

        let buf = file_patches.entry(path).or_insert_with(String::new);

        // Line content may not be valid UTF-8 (binary). Handle that gracefully.
        match str::from_utf8(line.content()) {
            Ok(s) => buf.push_str(s),
            Err(_) => buf.push_str("[BINARY DATA]\n"),
        }

        true // continue printing
    })?;

    let result: Vec<(PathBuf, String)> = file_patches.into_iter().collect();
    let (result_sources, result_diffs): (Vec<PathBuf>, Vec<String>) = result.into_iter().unzip();
    let mut expcommit = ExpandedCommit::new();
    if diff_profile.report_diffs {
        expcommit.diffs = Some(result_diffs);
    }
    if diff_profile.report_sources {
        expcommit.sources = Some(result_sources);
    }

    return Ok(expcommit);
}
