use crate::config::BranchAgainst;
use crate::config::{ContextFile, RvConfig};
use crate::git_helpers;
use crate::git_helpers::ExpandedCommit;
use crate::llm::{defs::LLMProvider, openai::OpenAIClient};
use crate::term_helpers;

use anyhow::{Context, Result};

use std::path::PathBuf;
use std::process;

const SYSTEM_PROMPT: &str = r#"
You are a senior software engineer and professional code reviewer. Produce a **concise, actionable, terminal-friendly** review of the code I provide. Follow these rules exactly:

OUTPUT FORMAT & STYLE
- Plain ASCII text only (no emojis, no markdown headers, no color codes).
- Wrap lines at ~80 columns.
- Keep output as short as possible while being actionable.
- Use short sentences. Prefer bullet/numbered lists.
- If there are no problems, print a one-line confirmation and a single short suggestion.

STRUCTURE (strict order)
1) FILE / CONTEXT: one line with filename or repo/PR identifier.
2) SUMMARY: 1 sentence describing overall quality & main issue (or "No issues found.").
3) SEVERITY: one word (CRITICAL, HIGH, MEDIUM, LOW, INFO).
4) FINDINGS: numbered list. Each finding: one-line title, then 1 short sentence explanation (max 2 sentences). Max 6 findings.
5) SUGGESTED FIX (per finding): For each finding give a minimal fix. Prefer a tiny unified-diff or 3-8 line code snippet. Label each fix with the finding number.
6) TESTS TO RUN: 1-3 bullet points telling how to validate the fix (commands or test ideas).
7) RISK / IMPACT: 1 line about backward-compatibility/perf/security impact.
8) ESTIMATED EFFORT: one word (Trivial / Small / Medium / Large).
9) FINAL VERDICT: one concise action sentence (e.g., "Approve", "Request changes: X", "Block: X").

CONTENT RULES
- Prioritize correctness, security, and maintainability in that order.
- If a line/variable is buggy, show the smallest concrete patch to fix it. Prefer exact code tokens over vague advice.
- Do not include long explanations or model apologetics.
- If multiple fixes are possible, give the simplest safe option first and mark alternatives as "Optional".
- If a finding is style-only, mark as INFO and give the project's typical lint rule suggestion (e.g., "run `cargo fmt` / `rustfmt`").
- When referencing lines, show the line snippet or diff context with line numbers if helpful, but keep it short.
- If you need runtime assumptions (platform, version), assume latest stable toolchain unless I say otherwise.
- When referencing the source code or when suggesting fixes or changes, ALWAYS write the correct path of the source file and the correct line number.
- Always consider the comments found in the source code, expecially if there are `[review]` or `[rv]` notes (they are directed to code review operators like you).
- NEVER report repetitions or redundancy
- NEVER report issues with the <diff> file that aren't present in the <source> file.
- ALWAYS check that your reviews are in line with the source codes provided.

NEGATIVE EXAMPLE:
```input
<source main.py>
a=10
</source>

<diff main.py>
-a=9
+a=10
</diff>
```

```output
There is a repetition in the a=10 statement.
```

POSITIVE EXAMPLE:
```input
<source main.py>
a=10
</source>

<diff main.py>
-a=9
+a=10
</diff>
```

```output
The code doesn't have any issues.
```

-------

{{custom_prompt}}
{{custom_guidelines}}

INPUT
- After this prompt I will provided an input formatted using:
    <diff FILE>   - tag used for submitting the diffs of a file
    <source FILE> - tag used for submitting the content of a file
    <info TYPE>   - tag used for additional info; can be of type README or CONTEXT
- Review this input.

--------
"#;
const CUSTOM_GUIDELINES_INTRO: &str = r#"
PROJECT GUIDELINES
"#;

fn read_context_files(context_file: ContextFile) -> Result<String, std::io::Error> {
    let filename = match context_file {
        ContextFile::Readme => "README.md",
        ContextFile::RvContext => ".rv_context",
        ContextFile::RvGuidelines => ".rv_guidelines",
    };

    match std::fs::read_to_string(filename) {
        Ok(content) => Ok(content),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(String::new()) // Return empty string if file doesn't exist
            } else {
                Err(e)
            }
        }
    }
}
pub fn pack_prompt_with_context(rvconfig: &RvConfig) -> String {
    let mut system_prompt = SYSTEM_PROMPT.to_string();

    // Handle custom guidelines
    let mut guidelines_content = String::new();
    if rvconfig.load_rv_guidelines {
        match read_context_files(ContextFile::RvGuidelines) {
            Ok(content) if !content.trim().is_empty() => {
                guidelines_content.push_str(CUSTOM_GUIDELINES_INTRO);
                guidelines_content.push_str(&content);
            }
            _ => {}
        }
    }
    system_prompt = system_prompt.replace("{{custom_guidelines}}", &guidelines_content);

    // Handle custom prompt
    let mut custom_prompt_content = String::new();
    if rvconfig.load_rv_context {
        match read_context_files(ContextFile::RvContext) {
            Ok(content) if !content.trim().is_empty() => {
                custom_prompt_content.push_str(&content);
            }
            _ => {}
        }
    }
    system_prompt = system_prompt.replace("{{custom_prompt}}", &custom_prompt_content);

    system_prompt
}

pub fn raw_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    file_path: Option<PathBuf>,
    dir_path: Option<PathBuf>,
    recursive: Option<bool>,
) {
    if file_path.is_some() {
        let path = file_path.unwrap();
        if !path.exists() {
            println!("[ERROR] File does not exist: {path:?}");
            return;
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
                process_review(&rvconfig, llm_selection, expcommit, None);
            }
            Err(e) => {
                println!("[ERROR] Failed to read file: {e}");
            }
        }
    } else if dir_path.is_some() {
        let path = dir_path.unwrap();
        if !path.exists() || !path.is_dir() {
            println!(
                "[ERROR] Directory does not exist or is not a directory: {path:?}"
            );
            return;
        }

        let recursive = recursive.unwrap_or(false);

        // Collect all files in directory
        let mut files = Vec::new();
        if let Err(e) = collect_files(&path, recursive, &mut files) {
            println!("[ERROR] Failed to collect files: {e}");
            return;
        }

        if files.is_empty() {
            println!("[ERROR] No files found in directory");
            return;
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
                    diffs.push(format!(
                        "[ERROR] Failed to read file {file_path:?}: {e}"
                    ));
                }
            }
        }

        expcommit.diffs = Some(diffs);
        process_review(&rvconfig, llm_selection, expcommit, None);
    } else {
        println!(
            "[ERROR] In order to use the RAW mode, you need to specify a --file or a --dir input"
        );
    }
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

fn process_review(
    rvconfig: &RvConfig,
    llm_selection: Option<String>,
    expcommit: ExpandedCommit,
    log_xml_structure: Option<bool>,
) {
    // Convert to structured format
    let review_prompt = expcommit.get_xml_structure(rvconfig.diff_profile);

    term_helpers::clear_term();
    if log_xml_structure.is_some() && log_xml_structure.unwrap() {
        println!("{review_prompt}");
        println!("  -------  ");
    }

    // Select correct LLM configuration and setup OpenAIClient
    let llm_configuration_default = rvconfig.clone().default_llm_config;
    let mut llm_configuration_key = llm_configuration_default;
    let llm_configs = rvconfig.clone().get_llm_configs();
    if llm_selection.is_some() {
        llm_configuration_key = llm_selection.unwrap();
    } else if !(llm_configs.contains_key(&llm_configuration_key.clone())) {
        println!(
            "[ERROR] No LLM configuration specified or wrong configuration specified; either create a `default`-named configuration or use the --llm parameter to change the configuration used."
        );
        process::exit(1);
    }
    let llm_configuration = match llm_configs.get(&llm_configuration_key.clone()) {
        Some(config) => config,
        None => {
            println!("[ERROR] Failed to load selected LLM configuration");
            process::exit(1);
        }
    };

    // Check if the API key is the placeholder or empty, and if it's OpenRouter, check for environment variable
    if llm_configuration.api_key == "[insert api key here]" || llm_configuration.api_key.is_empty()
    {
        if matches!(
            llm_configuration.provider,
            crate::config::OpenAIProvider::OpenRouter
        ) {
            if std::env::var("OPENROUTER_API_KEY").is_err() {
                println!(
                    "[ERROR] Insert compatible API key inside `~/.config/rv/config.toml` or set OPENROUTER_API_KEY environment variable"
                );
                process::exit(1);
            }
        } else {
            println!("[ERROR] Insert compatible API key inside `~/.config/rv/config.toml`");
            process::exit(1);
        }
    }

    let openai_client = OpenAIClient::from_config(llm_configuration.clone());

    // Build system prompt with context
    let system_prompt = pack_prompt_with_context(rvconfig);

    // Add README to the review prompt if configured
    let mut enhanced_review_prompt = review_prompt;
    if rvconfig.load_readme {
        match read_context_files(ContextFile::Readme) {
            Ok(readme_content) if !readme_content.trim().is_empty() => {
                enhanced_review_prompt.push_str("\n<info README>\n");
                enhanced_review_prompt.push_str(&readme_content);
                enhanced_review_prompt.push_str("\n</info>\n");
            }
            _ => {}
        }
    }

    openai_client.stream_request_stdout(system_prompt, enhanced_review_prompt);
}

pub fn git_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
    branch_mode: Option<BranchAgainst>,
    github_pr: Option<String>,
    log_xml_structure: Option<bool>,
) -> Result<()> {
    let mut expcommit: Option<ExpandedCommit> = None;

    if commit.is_some() {
        let commit_str = commit.unwrap();
        // TODO Better error handling
        let commit_oid = git_helpers::get_oid(&commit_str).context("Failed to get commit OID")?;
        let exp_result = git_helpers::expanded_from_commit(commit_oid);

        if exp_result.is_ok() {
            expcommit = Some(exp_result.unwrap());
        }
    } else if branch.is_some() {
        let mut used_branch_mode: BranchAgainst = rvconfig.default_branch_mode;
        if branch_mode.is_some() {
            used_branch_mode = branch_mode.unwrap();
        }
        let branch_name: String = branch.unwrap();

        let exp_result = git_helpers::expanded_from_branch(&branch_name, used_branch_mode);
        if exp_result.is_ok() {
            expcommit = Some(exp_result.unwrap());
        }
    } else if github_pr.is_some() {
        todo!("Github PR support");
    } else {
        // Staging edits, if empty HEAD commit
        let mut exp_result = git_helpers::staged_diffs(rvconfig.diff_profile);

        if exp_result.is_ok() {
            let exp_unwrapped: ExpandedCommit = exp_result.unwrap();

            if exp_unwrapped.clone().is_empty() {
                println!("Staged is empty, switching to HEAD");
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
            println!("Staged is empty, switching to HEAD");
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

    if expcommit.is_some() {
        process_review(
            &rvconfig,
            llm_selection,
            expcommit.unwrap(),
            log_xml_structure,
        );
    } else {
        println!("[ERROR] Git integrations failed. Are you running `rv` inside a Git repository?");
        println!("      | [LOG] {expcommit:?}");
    }

    Ok(())
}
