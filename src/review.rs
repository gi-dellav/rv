use crate::config::{ContextFile, RvConfig};
use crate::git_helpers;
use crate::llm::{defs::LLMProvider, openai::OpenAIClient};
use crate::term_helpers;

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
- [IMPORTANT] In order to say that something is redundant or repeated, check if it appears two or mode times in <source> tags; ignore <diff> tags for this check.

INPUT
- After this prompt I will provided an input formatted using:
    <diff FILE>   - tag used for submitting the diffs of a file
    <source FILE> - tag used for submitting the content of a file
    <info>        - tag used for additional info
- Review this input.

--------
"#;
const CUSTOM_GUIDELINES_INTRO: &str = r#"

PROJECT GUIDELINES
"#;

pub fn search_context_files(
  context_file: ContextFile
) -> PathBuf {
  todo!();
}

pub fn raw_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    file_path: Option<PathBuf>,
    dir_path: Option<PathBuf>,
    recursive: Option<bool>,
) {
    if file_path.is_some() {
        todo!("Raw file support");
    } else if dir_path.is_some() {
        todo!("Raw directory support");
    } else {
        println!(
            "[ERROR] In order to use the RAW mode, you need to specify a --file or a --dir input"
        );
    }
}

pub fn git_review(
    rvconfig: RvConfig,
    llm_selection: Option<String>,
    commit: Option<String>,
    branch: Option<String>,
    github_pr: Option<String>,
    log_xml_structure: Option<bool>,
) {
    if commit.is_some() {
        todo!("Git Commit support");
    } else if branch.is_some() {
        todo!("Git Branch support");
    } else if github_pr.is_some() {
        todo!("Github PR support");
    } else {
        // Staging edits
        let expcommit = git_helpers::staged_diffs(rvconfig.diff_profile);

        if expcommit.is_ok() {
            // Convert to structured format
            let review_prompt = expcommit.unwrap().get_xml_structure(rvconfig.diff_profile);

            term_helpers::clear_term();
            if log_xml_structure.is_some() {
                println!("{review_prompt}");
                println!("  -------  ");
            }

            // Select correct LLM configuration and setup OpenAIClient
            let llm_configuration_default = rvconfig.clone().default_llm_config; // Normally `default`
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
            let llm_configuration = llm_configs.get(&llm_configuration_key.clone()).unwrap();

            if llm_configuration.api_key == "[insert api key here]" {
                println!("[ERROR] Insert compatible API key inside `~/.config/rv/config.toml`");
                process::exit(1);
            }

            let openai_client = OpenAIClient::from_config(llm_configuration.clone());

            // TODO Custom Prompt support
            openai_client.stream_request_stdout(SYSTEM_PROMPT.to_string(), review_prompt);
        } else {
            println!(
                "[ERROR] Git integrations failed. Are you running `rv` inside a Git repository?"
            );
            println!("      | [LOG] {expcommit:?}");
        }
    }
}
