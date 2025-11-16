use crate::git_helpers::{self, ExpandedCommit};
use anyhow::{Context, Result, bail};
use git2::Oid;
use serde::Deserialize;
use serde_json;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct PrViewMetadata {
    number: u64,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
}

pub fn expanded_commit_from_pr(pr: &str) -> Result<ExpandedCommit> {
    ensure_gh_available()?;
    let metadata = fetch_pr_metadata(pr)?;

    ensure_base_available(&metadata.base_ref_name, &metadata.base_ref_oid)?;
    ensure_pr_head_available(metadata.number, &metadata.head_ref_oid)?;

    let base_oid = Oid::from_str(metadata.base_ref_oid.trim())
        .context("Invalid base commit SHA returned by gh")?;
    let head_oid = Oid::from_str(metadata.head_ref_oid.trim())
        .context("Invalid head commit SHA returned by gh")?;

    git_helpers::expanded_between_commits(base_oid, head_oid)
        .context("Failed to compute diff between PR base and head commits")
}

fn ensure_gh_available() -> Result<()> {
    let status = Command::new("gh")
        .arg("--version")
        .status()
        .context("Failed to invoke `gh --version`")?;

    if status.success() {
        Ok(())
    } else {
        bail!("GitHub CLI (gh) is not installed or not in PATH");
    }
}

fn fetch_pr_metadata(pr: &str) -> Result<PrViewMetadata> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            pr,
            "--json",
            "number,baseRefName,baseRefOid,headRefOid",
        ])
        .output()
        .context("Failed to invoke `gh pr view`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`gh pr view` failed: {stderr}");
    }

    serde_json::from_slice::<PrViewMetadata>(&output.stdout)
        .context("Unable to parse `gh pr view` JSON payload")
}

fn ensure_base_available(reference: &str, sha: &str) -> Result<()> {
    if commit_exists_locally(sha) {
        return Ok(());
    }

    let status = Command::new("git")
        .arg("fetch")
        .arg("origin")
        .arg(reference)
        .status()
        .context("Failed to invoke `git fetch` for PR base reference")?;

    if !status.success() {
        bail!("`git fetch origin {reference}` failed while preparing PR diff");
    }

    if commit_exists_locally(sha) {
        Ok(())
    } else {
        bail!("Base commit {sha} is still missing after fetch");
    }
}

fn ensure_pr_head_available(pr_number: u64, sha: &str) -> Result<()> {
    if commit_exists_locally(sha) {
        return Ok(());
    }

    let refspec = format!("pull/{pr_number}/head:refs/rv/pr/{pr_number}");
    let status = Command::new("git")
        .arg("fetch")
        .arg("origin")
        .arg(&refspec)
        .status()
        .context("Failed to invoke `git fetch` for PR head reference")?;

    if !status.success() {
        bail!("`git fetch origin {refspec}` failed while preparing PR diff");
    }

    if commit_exists_locally(sha) {
        Ok(())
    } else {
        bail!("Pull request head commit {sha} is still missing after fetch");
    }
}

fn commit_exists_locally(sha: &str) -> bool {
    Command::new("git")
        .args(["cat-file", "-e", &format!("{sha}^{{commit}}")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
