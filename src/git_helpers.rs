use crate::config::DiffProfile;
use git2::{Error, Commit, BranchType, Oid, Tree, DiffFormat, DiffOptions, Repository};
use std::{collections::{BTreeSet, HashMap}, env, fs, path::Path, path::PathBuf, str};

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

    /// Produce XML-like output useful for LLM prompting
    /// This operation should always be successful
    pub fn get_xml_structure(self, diff_profile: DiffProfile) -> String {
        let mut xml_string = String::new();
        // [review] I can unwrap because I can suppose that there are sources in order to generate a XML structure
        let sources = self.sources.as_ref().ok_or("Sources are missing").unwrap();

        if diff_profile.report_diffs {
            let mut diff_counter: usize = 0;
            // [review] I can unwrap beacuse I can suppose that there are diffs in order to generate a XML structure
            let diffs = self.diffs.as_ref().ok_or("Diffs are missing").unwrap();
            for diff_val in diffs {
                // Open <diff NAME> tag
                xml_string.push_str("<diff ");
                let diff_source_path = sources[diff_counter].to_string_lossy();
                xml_string.push_str(&diff_source_path);
                xml_string.push_str(" >\n");

                // Add diff
                xml_string.push_str(&diff_val);

                // Close </diff> tag
                xml_string.push_str("\n</diff>\n");

                diff_counter += 1;
            }
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
                let source_bytes = fs::read(&source_val).unwrap();
                let source_text = String::from_utf8_lossy(&source_bytes).to_string();
                xml_string.push_str(&source_text);

                // Close </source> tag
                xml_string.push_str("\n</source>\n");
            }
        }

        xml_string
    }
}

/// Get an ExpandedCommit rappresenting staged edits
/// TODO: Update to using `diff_trees_to_expanded`
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

        // Most .gitignore won't consider Cargo.lock, even tho it's not a good idea to include in the review prompt
        // In the future we might implement a more polished .rvignore file that works as a .gitignore counterpart for rv
        if !(path.to_str().unwrap().contains("Cargo.lock")) {
            let buf = file_patches.entry(path).or_default();

            // Line content may not be valid UTF-8 (binary). Handle that gracefully.
            match str::from_utf8(line.content()) {
                Ok(s) => buf.push_str(s),
                Err(_) => buf.push_str("[BINARY DATA]\n"),
            }
        }

        true // continue printing
    })?;

    let result: Vec<(PathBuf, String)> = file_patches.into_iter().collect();
    let (result_sources, result_diffs): (Vec<PathBuf>, Vec<String>) = result.into_iter().unzip();
    let mut expcommit = ExpandedCommit::new();
    if diff_profile.report_diffs {
        expcommit.diffs = Some(result_diffs);
    }
    // Keep the sources in order to allow ExpandedCommit::get_xml_structure to find the namefile of diffs
    // Don't worry, the report_sources variable will be considered in the get_xml_structure in order to allow source-less reports
    expcommit.sources = Some(result_sources);

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
        let maybe_path = delta.new_file().path().or(delta.old_file().path()).map(|p| p.to_path_buf());
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
        diffs: if patches.is_empty() { None } else { Some(patches) },
        sources: if touched.is_empty() { None } else { Some(touched.into_iter().collect()) },
    })
}

/// Build an ExpandedCommit for a given commit OID.
pub fn expanded_from_commit(repo: &Repository, oid: Oid) -> Result<ExpandedCommit, git2::Error> {
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
    diff_trees_to_expanded(repo, old_tree_ref, new_tree_ref)
}

/// Build an ExpandedCommit for HEAD (last commit on current branch).
pub fn expanded_from_head(repo: &Repository) -> Result<ExpandedCommit, git2::Error> {
    let head_ref = repo.head()?;
    let head_commit = head_ref.peel_to_commit()?;
    expanded_from_commit(repo, head_commit.id())
}

/// Enum to control what to compare a branch against
pub enum BranchAgainst {
    /// Compare branch against the current HEAD
    Current,
    /// Compare branch against the repository's `main`
    Main,
}

/// Try to find the `main` branch commit; fall back to `master` if `main` is not present.
fn find_main_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    // Try "refs/heads/main", then "refs/heads/master"
    if let Ok(branch) = repo.find_branch("main", BranchType::Local) {
        let commit = branch.into_reference().peel_to_commit()?;
        return Ok(commit);
    }
    if let Ok(branch) = repo.find_branch("master", BranchType::Local) {
        let commit = branch.into_reference().peel_to_commit()?;
        return Ok(commit);
    }
    // Not found: return an error from repo.head() to provide a git2::Error
    Err(Error::from_str("no 'main' or 'master' branch found"))
}

/// Compare the tip of `branch_name` against either the current HEAD or `main`/`master`.
/// Returns the diff between `base` (the `against` target) and the branch tip:
/// i.e., diff(base_tree, branch_tree) so the produced patches reflect changes from base -> branch.
pub fn expanded_from_branch_against(
    repo: &Repository,
    branch_name: &str,
    against: BranchAgainst,
) -> Result<ExpandedCommit, git2::Error> {
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
        BranchAgainst::Main => Some(find_main_commit(repo)?),
    };

    // get trees (Option<&Tree>)
    let new_tree = branch_commit.tree().ok();
    let old_tree = base_commit.as_ref().and_then(|c| c.tree().ok());

    let old_tree_ref = old_tree.as_ref();
    let new_tree_ref = new_tree.as_ref();

    diff_trees_to_expanded(repo, old_tree_ref, new_tree_ref)
}
