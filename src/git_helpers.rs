use crate::config::DiffProfile;
use git2::{DiffFormat, DiffOptions, Repository};
use std::{fs, collections::HashMap, path::PathBuf, str};

/// Structure that allow to contain both the diff and the edited source file for commits or for staged edits
#[derive(Clone, Debug)]
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

    /// Produce XML-like output useful for LLM prompting
    /// This operation should always be successful
    pub fn get_xml_structure(self, diff_profile: DiffProfile) -> String {
        let mut xml_string = String::new();
        let sources = self.sources.unwrap();

        if diff_profile.report_diffs {
            let mut diff_counter: usize = 0;
            let diffs = self.diffs.unwrap();
            for diff_val in diffs {
                // Open <diff NAME> tag
                xml_string.push_str("<diff ");
                let diff_source_path = sources[diff_counter].to_str().unwrap();
                xml_string.push_str(diff_source_path);
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
                let source_path = source_val.to_str().unwrap();
                xml_string.push_str(source_path);
                xml_string.push_str(" >\n");

                // Add source
                let source_text = fs::read_to_string(&source_val).unwrap();
                xml_string.push_str(&source_text);

                // Close </source> tag
                xml_string.push_str("\n</source>\n");
            }
        }

        return xml_string;
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

        // Most .gitignore won't consider Cargo.lock, even tho it's not a good idea to include in the review prompt
        // In the future we might implement a more polished .rvignore file that works as a .gitignore counterpart for rv
        if !(path.to_str().unwrap().contains("Cargo.lock")) {
            let buf = file_patches.entry(path).or_insert_with(String::new);

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

    return Ok(expcommit);
}
