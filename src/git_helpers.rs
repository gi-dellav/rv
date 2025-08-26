use crate::config::DiffProfile;
use git2::{DiffFormat, DiffOptions, Repository};
use std::{collections::HashMap, env, fs, path::Path, path::PathBuf, str};

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
