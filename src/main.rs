pub mod config;
pub mod git_helpers;
pub mod llm;
pub mod review;
pub mod term_helpers;

use clap::Parser;
use crate::review;
use crate::config;

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

    #[arg(short, long)]
    /// Github pull request to review
    pr: Option<String>,

    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    /// Specific file to review
    file: Option<String>,

    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    /// Specific directory to review
    dir: Option<String>,

    #[arg(short, long)]
    /// Review all subfiles, used with `--dir`
    recursive: Option<bool>,

    #[arg(long)]
    /// Review source code without interfacing with Git
    raw: Option<bool>,
}

fn main() {
    let args = Args::parse();

    println!("{:?}", args);
}
