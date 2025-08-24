pub mod config;
pub mod git_helpers;
pub mod llm;
pub mod review;
pub mod term_helpers;

pub use crate::config::RvConfig;

fn main() {
    let rvc: RvConfig = Default::default();

    println!("{rvc:?}");
    println!("\n\n----\n\n");
    println!("{}", toml::to_string_pretty(&rvc).unwrap());
}
