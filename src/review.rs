use crate::config::BranchAgainst;
use crate::config::RvConfig;
use crate::git_helpers;
use crate::git_helpers::ExpandedCommit;
use crate::github;
use crate::term_helpers;

use anyhow::{Context, Result};

use crate::llm::create_llm_provider;
use std::path::PathBuf;

const SYSTEM_PROMPT: &str = r#"
You are a senior software engineer and professional code reviewer.
Produce a concise, actionable, terminal-friendly review of the code
I provide. Follow these rules exactly.

OUTPUT FORMAT & STYLE
- ASCII only. No emojis, no markdown, no color codes.
- Soft-wrap at ~80 columns.
- Keep output minimal and actionable. Short sentences.
- Prefer numbered or bullet lists.
- If no problems: print one-line confirmation plus one short suggestion.

STRICT STRUCTURE (in this exact order)
1) FILE / CONTEXT: single line with filename or repo/PR id.
2) SUMMARY: one sentence describing overall quality & main issue or
   "No issues found."
3) SEVERITY: one word: CRITICAL (Security) / HIGH (Logic) / MEDIUM (Edge-case) / LOW (Optimization or style) / INFO.
4) FINDINGS: numbered list, max 6 items. Each item: one-line title,
   then 1 short sentence explanation (<=2 sentences).
5) SUGGESTED FIX [per finding]: minimal fix for each finding. Prefer
   a tiny unified-diff or a 3–8 line code snippet. Label fixes with
   the finding number.
6) TESTS TO RUN: 1–3 bullets with exact commands or test ideas.
7) RISK / IMPACT: one line about backward-compat, perf, security.
8) ESTIMATED EFFORT: one word: Trivial / Small / Medium / Large.
9) FINAL VERDICT: one concise action sentence (e.g., "Approve",
   "Request changes: X", "Block: X").

KEY RULES (must obey)
- Prioritize correctness, security, maintainability (in that order).
- If a line/variable is buggy, provide the smallest concrete patch.
  Prefer exact code tokens over vague advice.
- If multiple safe fixes exist, give the simplest first. Mark others
  as "Optional".
- Style-only issues: mark as INFO and name the lint command.
- Always include the exact source file path and line number when
  referencing code or suggesting edits.
- Respect comments in source, especially tags like [review] or [rv].
- MAX 6 findings. Do not add filler text or apologies.
- NEVER report repetitions or diffs that don't exist in the source.
- NEVER include issues about the <diff> that aren't present in the
  <source>.
- Assume latest stable toolchain unless told otherwise.

INPUT FORMAT (what I'll send next)
- <context FILE>   : text file containing context about the project
- <guideline FILE> : text file containing guidelines and instructions
- <diff FILE>      : git diff of the file to review
- <source FILE>    : text file to be reviewed

Now review the input I will provide next. Produce the review using the
exact structure and rules above.


=============================

"#;

fn read_file(filename: &str) -> Option<String> {
    // TODO Load files from project's root directory not current working directory
    match std::fs::read_to_string(filename) {
        Ok(content) => Some(content),
        Err(_) => None,
    }
}
/// Add context, guidelines and custom instructions to the LLM prompt
pub fn pack_prompt(rvconfig: &RvConfig) -> Result<String> {
    let mut system_prompt = SYSTEM_PROMPT.to_string();
    let mut suffix_context: String = String::new();

    // Handle project guidelines files
    for f in rvconfig.project_guidelines_files.files.clone() {
        let content = read_file(&f);
        if content.is_some() {
            suffix_context.push_str(&format!("<guideline {f}>"));
            suffix_context.push_str(&content.unwrap_or_default());
            suffix_context.push_str("</guideline>");
        }
    }

    // Handle project context files
    for f in rvconfig.project_context_files.files.clone() {
        let content = read_file(&f);
        if content.is_some() {
            suffix_context.push_str(&format!("<context {f}>"));
            suffix_context.push_str(&content.unwrap_or_default());
            suffix_context.push_str("</context>");
        }
    }

    // Handle custom prompts
    // TODO Implement custom prompts support

    system_prompt.push_str(&suffix_context);

    Ok(system_prompt)
}

pub async fn raw_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    file_path: Option<PathBuf>,
    dir_path: Option<PathBuf>,
    recursive: Option<bool>,
    pipe: bool,
) -> Result<()> {
    if let Some(path) = file_path {
        if !path.exists() {
            println!("[ERROR] File does not exist: {path:?}");
            return Ok(());
        }

        // Create ExpandedCommit structure for single file
        let mut expcommit = ExpandedCommit::new();
        expcommit.sources = Some(vec![path.clone()]);

        // Read file content
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if rvconfig.diff_profile.report_sources {
                    // For raw mode, we'll treat the file content as both source and "diff"
                    // Since there's no actual diff, we can show the entire file
                    expcommit.diffs = Some(vec![format!("Raw file content:\n{}", content)]);
                } else {
                    expcommit.diffs =
                        Some(vec![String::from("File content not shown in diff mode")]);
                }

                // Process the review
                process_review(&rvconfig, llm_selection, expcommit, None, pipe).await?;
            }
            Err(e) => {
                println!("[ERROR] Failed to read file: {e}");
                return Err(e.into());
            }
        }
    } else if let Some(path) = dir_path {
        if !path.exists() || !path.is_dir() {
            println!("[ERROR] Directory does not exist or is not a directory: {path:?}");
            return Ok(());
        }

        let recursive = recursive.unwrap_or(false);

        // Collect all files in directory
        let mut files = Vec::new();
        if let Err(e) = collect_files(&path, recursive, &mut files) {
            println!("[ERROR] Failed to collect files: {e}");
            return Err(e.into());
        }

        if files.is_empty() {
            println!("[ERROR] No files found in directory");
            return Ok(());
        }

        // Create ExpandedCommit structure for directory
        let mut expcommit = ExpandedCommit::new();
        expcommit.sources = Some(files.clone());

        // Read all file contents
        let mut diffs = Vec::new();
        for file_path in files {
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    if rvconfig.diff_profile.report_sources {
                        diffs.push(format!("File: {file_path:?}\n{content}"));
                    } else {
                        diffs.push(format!("File: {file_path:?} (content not shown)"));
                    }
                }
                Err(e) => {
                    diffs.push(format!("[ERROR] Failed to read file {file_path:?}: {e}"));
                }
            }
        }

        expcommit.diffs = Some(diffs);
        process_review(&rvconfig, llm_selection, expcommit, None, pipe).await?;
    } else {
        println!(
            "[ERROR] In order to use the RAW mode, you need to specify a --file or a --dir input"
        );
    }
    Ok(())
}

fn collect_files(
    dir: &PathBuf,
    recursive: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                collect_files(&path, recursive, files)?;
            }
        } else {
            files.push(path);
        }
    }
    Ok(())
}

async fn process_review(
    rvconfig: &RvConfig,
    llm_selection: Option<String>,
    expcommit: ExpandedCommit,
    log_xml_structure: Option<bool>,
    pipe: bool,
) -> Result<()> {
    // Convert to structured format
    let review_prompt = expcommit.get_xml_structure(rvconfig.diff_profile);

    if !pipe {
        term_helpers::clear_term();
    }
    if log_xml_structure.is_some() && log_xml_structure.unwrap() {
        println!("{review_prompt}");
        println!("  -------  ");
    }

    // Select correct LLM configuration
    let llm_configuration_default = rvconfig.clone().default_llm_config;
    let mut llm_configuration_key = llm_configuration_default;
    let llm_configs = rvconfig.clone().get_llm_configs();
    if let Some(selection) = llm_selection {
        llm_configuration_key = selection;
    } else if !(llm_configs.contains_key(&llm_configuration_key.clone())) {
        println!(
            "[ERROR] No LLM configuration specified or wrong configuration specified; either create a `default`-named configuration or use the --llm parameter to change the configuration used."
        );
        std::process::exit(1);
    }
    let llm_configuration = match llm_configs.get(&llm_configuration_key.clone()) {
        Some(config) => config,
        None => {
            println!("[ERROR] Failed to load selected LLM configuration");
            std::process::exit(1);
        }
    };

    let api_key = llm_configuration.resolve_api_key()?;
    let system_prompt = pack_prompt(rvconfig)?;

    // Create LLM provider using factory pattern
    let mut llm_config_with_key = llm_configuration.clone();
    llm_config_with_key.api_key = api_key;
    let client = create_llm_provider(llm_config_with_key);

    // Stream the response to stdout
    client.stream_request_stdout(system_prompt, review_prompt)?;

    Ok(())
}

pub async fn git_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
    branch_mode: Option<BranchAgainst>,
    github_pr: Option<String>,
    log_xml_structure: Option<bool>,
    pipe: bool,
) -> Result<()> {
    let mut expcommit: Option<ExpandedCommit> = None;

    if let Some(commit_str) = commit {
        //println!("[DEBUG] Reviewing commit: {}", commit_str);
        let commit_oid = git_helpers::get_oid(&commit_str).context("Failed to get commit OID")?;
        let exp_result = git_helpers::expanded_from_commit(commit_oid);

        if exp_result.is_ok() {
            expcommit = Some(exp_result.unwrap());
        }
    } else if let Some(branch_name) = branch {
        //println!("[DEBUG] Reviewing branch: {}", branch_name);
        let mut used_branch_mode: BranchAgainst = rvconfig.default_branch_mode;
        if let Some(mode) = branch_mode {
            used_branch_mode = mode;
        }

        let exp_result = git_helpers::expanded_from_branch(&branch_name, used_branch_mode);
        if exp_result.is_ok() {
            expcommit = Some(exp_result.unwrap());
        }
    } else if let Some(pr_id) = github_pr {
        //println!("[DEBUG] Reviewing GitHub PR: {}", pr_id);
        let pr_expcommit = github::expanded_commit_from_pr(&pr_id)
            .context("Failed to build diff from GitHub pull request")?;
        expcommit = Some(pr_expcommit);
    } else {
        //println!("[DEBUG] Reviewing staged changes or HEAD");
        // Staging edits, if empty HEAD commit
        let mut exp_result = git_helpers::staged_diffs(rvconfig.diff_profile);

        if exp_result.is_ok() {
            let exp_unwrapped: ExpandedCommit = exp_result.unwrap();

            if exp_unwrapped.clone().is_empty() {
                if !pipe {
                    println!("Staged is empty, switching to HEAD");
                }
                let commit_str = "HEAD";
                // TODO Better error handling
                let commit_oid =
                    git_helpers::get_oid(commit_str).context("Failed to get commit OID")?;
                exp_result = git_helpers::expanded_from_commit(commit_oid);

                if exp_result.is_ok() {
                    expcommit = Some(exp_result.unwrap());
                }
            } else {
                expcommit = Some(exp_unwrapped);
            }
        } else {
            // HEAD commit
            if !pipe {
                println!("Staged is empty, switching to HEAD");
            }
            let commit_str = "HEAD";
            // TODO Better error handling
            let commit_oid =
                git_helpers::get_oid(commit_str).context("Failed to get commit OID")?;
            exp_result = git_helpers::expanded_from_commit(commit_oid);

            if exp_result.is_ok() {
                expcommit = Some(exp_result.unwrap());
            }
        }
    }

    if let Some(expanded) = expcommit {
        process_review(&rvconfig, llm_selection, expanded, log_xml_structure, pipe).await?;
    } else {
        println!("[ERROR] Git integrations failed. Are you running `rv` inside a Git repository?");
        println!("      | [LOG] {expcommit:?}");
    }

    Ok(())
}
