pub mod config;
pub mod git_helpers;
pub mod github;
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
    commit: Option<String>,

    #[arg(short, long)]
    /// Git branch to review
    branch: Option<String>,

    #[arg(long, value_enum)]
    /// Git branch review mode
    branch_mode: Option<config::BranchAgainst>,

    #[arg(short, long)]
    /// Github pull request to review
    pr: Option<String>,

    #[arg(long = "log_xml", action)]
    /// Print out XML structure of the code review.
    log_xml_structure: bool,

    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    /// Specific file to review
    file: Option<PathBuf>,

    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    /// Specific directory to review
    dir: Option<PathBuf>,

    #[arg(short, long, action)]
    /// Review all subfiles, used with `--dir`
    recursive: bool,

    #[arg(short = 'R', long)]
    /// Review source code without interfacing with Git
    raw: bool,

    #[arg(short = 'P', long, action)]
    /// Output as raw text, allowing for stdout pipes
    pipe: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let rvconfig = config::RvConfig::load_default().unwrap();

    if args.raw {
        if let Err(e) =
            review::raw_review(rvconfig, args.llm, args.file, args.dir, Some(args.recursive), args.pipe).await
        {
            eprintln!("Error during raw review: {e}");
            std::process::exit(1);
        }
    } else {
        // Check that only 0 or 1 arguments between commit, branch or pr are used
        // In order to make it smaller, it turns boolean values to u8 and sums them in order to get the number of enabled args
        let enabled_git_args: u8 =
            args.commit.is_some() as u8 + args.branch.is_some() as u8 + args.pr.is_some() as u8;

        if enabled_git_args > 1 {
            println!(
                "[ERROR] You can enable only one parameter between --commit, --branch or --pr"
            );
        } else if let Err(e) = review::git_review(
            rvconfig,
            args.llm,
            args.commit,
            args.branch,
            args.branch_mode,
            args.pr,
            Some(args.log_xml_structure),
            args.pipe,
        )
        .await
        {
            eprintln!("Error during code review: {e}");
            std::process::exit(1);
        }
    }
}
