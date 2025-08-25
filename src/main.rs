pub mod config;
pub mod git_helpers;
pub mod llm;
pub mod review;
pub mod term_helpers;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// LLM configuration to use
    llm: Option<String>,

    #[arg(short, long)]
    /// Git commit to review
    commit: Option<String>, //TODO

    #[arg(short, long)]
    /// Git branch to review
    branch: Option<String>, //TODO

    #[arg(short, long)]
    /// Github pull request to review
    pr: Option<String>, //TODO

    #[arg(long)]
    /// Print out XML structure of the code review.
    log_xml_structure: Option<bool>,

    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    /// Specific file to review
    file: Option<PathBuf>,

    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    /// Specific directory to review
    dir: Option<PathBuf>,

    #[arg(short, long)]
    /// Review all subfiles, used with `--dir`
    recursive: Option<bool>,

    #[arg(long)]
    /// Review source code without interfacing with Git
    raw: Option<bool>,
}

fn main() {
    let args = Args::parse();
    let rvconfig = config::RvConfig::load_default().unwrap();
    let raw_mode = args.raw.unwrap_or(false);

    if raw_mode {
        review::raw_review(rvconfig, args.llm, args.file, args.dir, args.recursive);
    } else {
        // Check that only 0 or 1 arguments between commit, branch or pr are used
        // In order to make it smaller, it turns boolean values to u8 and sums them in order to get the number of enabled args
        let enabled_git_args: u8 =
            args.commit.is_some() as u8 + args.branch.is_some() as u8 + args.pr.is_some() as u8;

        if enabled_git_args > 1 {
            println!(
                "[ERROR] You can enable only one parameter between --commit, --branch or --pr"
            );
        } else {
            review::git_review(
                rvconfig,
                args.llm,
                args.commit,
                args.branch,
                args.pr,
                args.log_xml_structure,
            );
        }
    }
}
