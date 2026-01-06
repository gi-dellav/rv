use crate::config::{BranchAgainst, DiffProfile};
use git2::Object;
use git2::{BranchType, Commit, DiffFormat, DiffOptions, Error, Oid, Repository, Tree};
use std::{collections::BTreeSet, env, fs, path::Path, path::PathBuf, str};

/// Structure that allow to contain both the diff and the edited source file for commits or for staged edits
#[derive(Clone, Debug)]
pub struct ExpandedCommit {
    //pub workdir: String,
    pub diffs: Option<Vec<String>>,
    pub sources: Option<Vec<PathBuf>>,
}
impl Default for ExpandedCommit {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpandedCommit {
    pub fn new() -> ExpandedCommit {
        ExpandedCommit {
            diffs: None,
            sources: None,
        }
    }

    pub fn is_empty(self) -> bool {
        // Sources must exist and have at least 1 element
        if self.sources.is_some() && self.sources.unwrap().len() > 0 {
            return false;
        }
        true
    }

    /// Produce XML-like output useful for LLM prompting
    /// This operation should always be successful
    pub fn get_xml_structure(self, diff_profile: DiffProfile) -> String {
        let mut xml_string = String::new();
        let sources = self.sources.as_ref().ok_or("Sources are missing").unwrap();

        if diff_profile.report_diffs {
            let diffs = self.diffs.as_ref().ok_or("Diffs are missing").unwrap();
            for (diff_counter, diff_val) in diffs.iter().enumerate() {
                // Open <diff NAME> tag
                xml_string.push_str("<diff ");
                let diff_source_path = sources[diff_counter].to_string_lossy();
                xml_string.push_str(&diff_source_path);
                xml_string.push_str(" >\n");

                // Add diff
                xml_string.push_str(diff_val);

                // Close </diff> tag
                xml_string.push_str("\n</diff>\n");
            }
        }
        if diff_profile.report_diffs && diff_profile.report_sources {
            println!("\n\n---SPLIT----\n--Do not confuse text before and after the SPLIT--\n\n");
        }
        if diff_profile.report_sources {
            for source_val in sources {
                // Open <source NAME> tag
                xml_string.push_str("<source ");
                // [review] Ignore this line, .to_string_lossy is the correct choice
                let source_path = source_val.to_string_lossy();
                xml_string.push_str(&source_path);
                xml_string.push_str(" >\n");

                // Add source
                match fs::read(source_val) {
                    Ok(source_bytes) => {
                        let source_text = String::from_utf8_lossy(&source_bytes);
                        xml_string.push_str(&source_text);
                    }
                    Err(err) => {
                        xml_string.push_str(&format!("[source unavailable: {err}]"));
                    }
                }

                // Close </source> tag
                xml_string.push_str("\n</source>\n");
            }
        }

        xml_string
    }
}

/// Get an ExpandedCommit representing staged edits
pub fn staged_diffs(diff_profile: DiffProfile) -> Result<ExpandedCommit, git2::Error> {
    let repo = Repository::discover(".")?;
    let index = repo.index()?;

    // Set cwd to repository main directory
    let workdir: &Path = repo
        .workdir()
        .ok_or("Bare repository has no working directory")
        .unwrap();
    env::set_current_dir(workdir).unwrap();

    // Try to get HEAD tree. If repo has no commits yet, treat HEAD tree as None.
    let head_tree = match repo.head() {
        Ok(reference) => Some(reference.peel_to_tree()?),
        Err(_) => None,
    };

    let mut diff_opts = DiffOptions::new();
    // Customize diff_opts if you want (context lines, pathspecs, etc.)
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), Some(&mut diff_opts))?;

    // Use the same logic as diff_trees_to_expanded to collect patches and touched files
    let mut patches: Vec<String> = Vec::new();
    let mut current_patch = String::new();
    let mut last_file: Option<PathBuf> = None;
    let mut touched: BTreeSet<PathBuf> = BTreeSet::new();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        // Determine the file path for this delta: prefer the new file path, else old file path
        let maybe_path = delta
            .new_file()
            .path()
            .or(delta.old_file().path())
            .map(|p| p.to_path_buf());
        // If the delta changed (a new file's patch started), flush the previous patch
        if last_file.as_ref() != maybe_path.as_ref() {
            if !current_patch.is_empty() {
                patches.push(std::mem::take(&mut current_patch));
            }
            last_file = maybe_path.clone();
        }

        // Append the line content (may be binary; try to decode as UTF-8)
        let content = line.content();
        match std::str::from_utf8(content) {
            Ok(s) => current_patch.push_str(s),
            Err(_) => current_patch.push_str(&format!("<non-utf8 {} bytes>", content.len())),
        }

        if let Some(p) = maybe_path {
            touched.insert(p);
        }
        // return true to continue processing
        true
    })?;

    // push the last accumulated patch if any
    if !current_patch.is_empty() {
        patches.push(current_patch);
    }

    // Filter out Cargo.lock from both patches and touched files
    let mut filtered_patches = Vec::new();
    let mut filtered_touched = BTreeSet::new();
    for (patch, path) in patches.into_iter().zip(touched.into_iter()) {
        if let Some(path_str) = path.to_str()
            && path_str.contains("Cargo.lock")
        {
            continue;
        }
        filtered_patches.push(patch);
        filtered_touched.insert(path);
    }

    let mut expcommit = ExpandedCommit::new();
    if diff_profile.report_diffs && !filtered_patches.is_empty() {
        expcommit.diffs = Some(filtered_patches);
    }
    // Keep the sources in order to allow ExpandedCommit::get_xml_structure to find the namefile of diffs
    // Don't worry, the report_sources variable will be considered in the get_xml_structure in order to allow source-less reports
    expcommit.sources = if filtered_touched.is_empty() {
        None
    } else {
        Some(filtered_touched.into_iter().collect())
    };

    Ok(expcommit)
}

fn diff_trees_to_expanded(
    repo: &Repository,
    old_tree: Option<&Tree>,
    new_tree: Option<&Tree>,
) -> Result<ExpandedCommit, git2::Error> {
    let diff = repo.diff_tree_to_tree(old_tree, new_tree, None)?;
    // Collect patches (one string per file) and touched files
    let mut patches: Vec<String> = Vec::new();
    let mut current_patch = String::new();
    let mut last_file: Option<PathBuf> = None;
    let mut touched: BTreeSet<PathBuf> = BTreeSet::new();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        // Determine the file path for this delta: prefer the new file path, else old file path
        let maybe_path = delta
            .new_file()
            .path()
            .or(delta.old_file().path())
            .map(|p| p.to_path_buf());
        // If the delta changed (a new file's patch started), flush the previous patch
        if last_file.as_ref() != maybe_path.as_ref() {
            if !current_patch.is_empty() {
                patches.push(std::mem::take(&mut current_patch));
            }
            last_file = maybe_path.clone();
        }

        // Append the line content (may be binary; try to decode as UTF-8)
        let content = line.content();
        match std::str::from_utf8(content) {
            Ok(s) => current_patch.push_str(s),
            Err(_) => current_patch.push_str(&format!("<non-utf8 {} bytes>", content.len())),
        }

        if let Some(p) = maybe_path {
            touched.insert(p);
        }
        // return true to continue processing
        true
    })?;

    // push the last accumulated patch if any
    if !current_patch.is_empty() {
        patches.push(current_patch);
    }

    Ok(ExpandedCommit {
        diffs: if patches.is_empty() {
            None
        } else {
            Some(patches)
        },
        sources: if touched.is_empty() {
            None
        } else {
            Some(touched.into_iter().collect())
        },
    })
}

/// Build an ExpandedCommit for a given commit OID.
pub fn expanded_from_commit(oid: Oid) -> Result<ExpandedCommit, git2::Error> {
    let repo = Repository::discover(".")?;
    let commit = repo.find_commit(oid)?;
    let new_tree = commit.tree().ok();
    // parent tree (if any)
    let old_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    let old_tree_ref = old_tree.as_ref();
    let new_tree_ref = new_tree.as_ref();
    diff_trees_to_expanded(&repo, old_tree_ref, new_tree_ref)
}

/// Build an ExpandedCommit for HEAD (last commit on current branch).
pub fn expanded_from_head() -> Result<ExpandedCommit, git2::Error> {
    let repo = Repository::discover(".")?;
    let head_ref = repo.head()?;
    let head_commit = head_ref.peel_to_commit()?;
    expanded_from_commit(head_commit.id())
}

/// Compare the tip of `branch_name` against either the current HEAD or `main`/`master`.
/// Returns the diff between `base` (the `against` target) and the branch tip:
/// i.e., diff(base_tree, branch_tree) so the produced patches reflect changes from base -> branch.
pub fn expanded_from_branch(
    branch_name: &str,
    against: BranchAgainst,
) -> Result<ExpandedCommit, git2::Error> {
    let search_repo = Repository::discover(".")?;
    let repo = Repository::discover(".")?;
    // Find branch commit
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let branch_commit = branch.into_reference().peel_to_commit()?;

    // Determine base commit to compare against
    let base_commit: Option<Commit> = match against {
        BranchAgainst::Current => {
            // If HEAD is unborn (no commits), repo.head() may fail; handle by returning an error
            let head_ref = repo.head()?;
            Some(head_ref.peel_to_commit()?)
        }
        BranchAgainst::Main => {
            if let Ok(branch) = search_repo.find_branch("main", BranchType::Local) {
                let commit = branch.into_reference().peel_to_commit()?;
                Some(commit)
            } else if let Ok(branch) = search_repo.find_branch("master", BranchType::Local) {
                let commit = branch.into_reference().peel_to_commit()?;
                Some(commit)
            } else {
                panic!(
                    "[ERR] Tried to compare against the main branch, but there are no branches named 'main' or 'master'."
                );
            }
        }
    };

    // get trees (Option<&Tree>)
    let new_tree = branch_commit.tree().ok();
    let old_tree = base_commit.as_ref().and_then(|c| c.tree().ok());

    let old_tree_ref = old_tree.as_ref();
    let new_tree_ref = new_tree.as_ref();

    diff_trees_to_expanded(&repo, old_tree_ref, new_tree_ref)
}

pub fn expanded_between_commits(
    base_oid: Oid,
    head_oid: Oid,
) -> Result<ExpandedCommit, git2::Error> {
    let repo = Repository::discover(".")?;
    let base_commit = repo.find_commit(base_oid)?;
    let head_commit = repo.find_commit(head_oid)?;
    let base_tree = base_commit.tree().ok();
    let head_tree = head_commit.tree().ok();

    let base_tree_ref = base_tree.as_ref();
    let head_tree_ref = head_tree.as_ref();

    diff_trees_to_expanded(&repo, base_tree_ref, head_tree_ref)
}

pub fn get_oid(rev: &str) -> Result<Oid, Error> {
    let repo = Repository::discover(".")?;
    // If the input parses as an Oid, try that first (fast path).
    if let Ok(oid) = Oid::from_str(rev) {
        if let Ok(obj) = repo.find_object(oid, None) {
            // If the object (or what it points to) is a commit, return its id.
            if let Ok(commit) = obj.peel_to_commit() {
                return Ok(commit.id());
            }
            // If the parsed OID is not a commit, fall through to rev-parse fallback.
        } else {
            // If find_object failed for this OID, fallthrough to revparse (covers packed/loose mismatch).
        }
    }

    // Fallback: rev-parse (handles short hashes, refs, HEAD~, tags, etc.)
    let obj: Object = repo.revparse_single(rev)?;
    let commit = obj.peel_to_commit()?;
    Ok(commit.id())
}

/// Get the OID of the parent commit of a given commit
pub fn get_parent_oid(commit_oid: Oid) -> Result<Oid, Error> {
    let repo = Repository::discover(".")?;
    let commit = repo.find_commit(commit_oid)?;
    if commit.parent_count() > 0 {
        Ok(commit.parent(0)?.id())
    } else {
        Err(Error::from_str("Commit has no parent"))
    }
}
