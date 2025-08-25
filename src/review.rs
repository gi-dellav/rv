use crate::config::RvConfig;
use crate::git_helpers;
use crate::llm;
use crate::term_helpers;

use std::path::PathBuf;

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

INPUT
- After this prompt I will provided an input formatted using:
    <diff FILE>   - tag used for submitting the diffs of a file
    <source FILE> - tag used for submitting the content of a file
    <info>        - tag used for additional info
- Review this input.

--------
"#;

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

          println!("{}", review_prompt);
        } else {
          println!("[ERROR] Git integrations failed. Are you running `rv` inside a Git repository?");
          println!("      | [LOG] {:?}", expcommit);
        }
    }
}
