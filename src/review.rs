use crate::config::RvConfig;
use crate::git_helpers;
use crate::llm;
use crate::term_helpers;

use std::path::PathBuf;

pub fn raw_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    file_path: Option<PathBuf>,
    dir_path: Option<PathBuf>,
    recursive: Option<bool>,
) {
}

pub fn git_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
    github_pr: Option<String>,
) {
}
