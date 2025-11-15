use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::path::PathBuf;

pub fn default_config_path() -> io::Result<PathBuf> {
    // Build the path
    let path: PathBuf = if let Some(mut dir) = dirs::config_dir() {
        dir.push("rv");
        dir.push("config.toml");
        dir
    } else {
        // Fallback: $HOME/.config/rv/config.toml
        let home = std::env::var_os("HOME").ok_or_else(|| {
            io::Error::new(
                ErrorKind::NotFound,
                "could not determine config directory (no XDG config dir and HOME not set)",
            )
        })?;
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("rv");
        p.push("config.toml");
        p
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    } else {
        return Err(io::Error::other("config path has no parent directory"));
    }

    Ok(path)
}

// --- serde default helpers --------------------------------------------------

fn default_report_diffs() -> bool {
    true
}

fn default_report_sources() -> bool {
    true
}

fn default_configuration_name() -> String {
    "default".to_string()
}

fn default_openai_provider() -> OpenAIProvider {
    OpenAIProvider::OpenRouter
}

fn default_model_id() -> String {
    "deepseek/deepseek-r1:free".to_string()
}

fn default_api_key() -> String {
    "[insert api key here]".to_string()
}

fn default_allow_reasoning() -> bool {
    true
}

fn default_llm_configs() -> Vec<LLMConfig> {
    vec![
        LLMConfig {
            configuration_name: String::from("default"),
            provider: default_openai_provider(),
            model_id: String::from("deepseek/deepseek-r1-distill-qwen-32b"),
            api_key: default_api_key(),
            allow_reasoning: true,
            custom_prompt: None,
        },
        LLMConfig {
            configuration_name: String::from("think"),
            provider: default_openai_provider(),
            model_id: String::from("deepseek/deepseek-r1"),
            api_key: default_api_key(),
            allow_reasoning: true,
            custom_prompt: None,
        },
    ]
}

fn default_default_llm_config() -> String {
    "default".to_string()
}

fn default_branch_mode() -> BranchAgainst {
    BranchAgainst::Main
}

fn default_load_readme() -> bool {
    true
}

fn default_load_rv_context() -> bool {
    true
}

fn default_load_rv_guidelines() -> bool {
    true
}

// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(default)]
/// How the LLM context gets produced
pub struct DiffProfile {
    #[serde(default = "default_report_diffs")]
    pub report_diffs: bool,
    #[serde(default = "default_report_sources")]
    pub report_sources: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CustomPrompt {
    Suffix(String),
    Replace(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
/// LLM provider specific configuration
pub struct LLMConfig {
    #[serde(default = "default_configuration_name")]
    pub configuration_name: String,
    #[serde(default = "default_openai_provider")]
    pub provider: OpenAIProvider,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_api_key")]
    pub api_key: String,

    // TODO: Implement optional reasioning
    #[serde(default = "default_allow_reasoning")]
    pub allow_reasoning: bool,

    #[serde(default)]
    pub custom_prompt: Option<CustomPrompt>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
/// Main configuration structure, used in `~/.config/rv/config.toml`
pub struct RvConfig {
    #[serde(default)]
    pub diff_profile: DiffProfile,
    #[serde(default = "default_llm_configs")]
    pub llm_configs: Vec<LLMConfig>,
    #[serde(default = "default_default_llm_config")]
    pub default_llm_config: String,
    #[serde(default = "default_branch_mode")]
    pub default_branch_mode: BranchAgainst,
    #[serde(default = "default_load_readme")]
    pub load_readme: bool,
    #[serde(default = "default_load_rv_context")]
    pub load_rv_context: bool,
    #[serde(default = "default_load_rv_guidelines")]
    pub load_rv_guidelines: bool,
}

// -----------------------------------

impl Default for DiffProfile {
    fn default() -> Self {
        DiffProfile {
            report_diffs: true,
            report_sources: true,
        }
    }
}

// test

impl Default for LLMConfig {
    fn default() -> Self {
        LLMConfig {
            configuration_name: String::from("default"),
            provider: OpenAIProvider::OpenRouter,
            model_id: String::from("deepseek/deepseek-r1:free"),
            api_key: String::from("[insert api key here]"),
            allow_reasoning: true,
            custom_prompt: None,
        }
    }
}

impl Default for RvConfig {
    fn default() -> Self {
        let diff_profile: DiffProfile = Default::default();
        let llm_configs = default_llm_configs();

        RvConfig {
            diff_profile,
            llm_configs,
            default_llm_config: String::from("default"),
            default_branch_mode: BranchAgainst::Main,
            load_readme: true,
            load_rv_context: true,
            load_rv_guidelines: true,
        }
    }
}

impl RvConfig {
    pub fn load_from_path(path: String) -> anyhow::Result<RvConfig> {
        let mut file = File::open(&path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config: RvConfig = toml::from_str(&contents)?;

        Ok(config)
    }

    pub fn load_default() -> anyhow::Result<RvConfig> {
        let config_path = default_config_path()?;
        let loaded_config: anyhow::Result<RvConfig> =
            RvConfig::load_from_path(config_path.display().to_string());

        if loaded_config.is_ok() {
            // Return succesfully loaded config
            Ok(loaded_config.unwrap())
        } else {
            // Create new config
            let new_config: RvConfig = Default::default();

            // Save to disk as config.toml
            let toml_string = toml::to_string_pretty(&new_config)?;
            fs::write(config_path, toml_string)?;

            Ok(new_config)
        }
    }

    pub fn get_llm_configs(self) -> HashMap<String, LLMConfig> {
        let mut llm_hashmap: HashMap<String, LLMConfig> = HashMap::new();

        for lc in self.llm_configs {
            llm_hashmap.insert(lc.configuration_name.clone(), lc.clone());
        }

        llm_hashmap
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum OpenAIProvider {
    OpenAI,
    OpenRouter,
}

impl Default for OpenAIProvider {
    fn default() -> Self {
        OpenAIProvider::OpenRouter
    }
}

impl OpenAIProvider {
    pub fn get_endpoint(self) -> String {
        match self {
            OpenAIProvider::OpenAI => String::from("https://api.openai.com/v1"),
            OpenAIProvider::OpenRouter => String::from("https://openrouter.ai/api/v1"),
        }
    }
}

/// Enum to indicate a certain standard file used for providing extra context
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ContextFile {
    Readme,
    RvContext,
    RvGuidelines,
}

/// Enum to control what to compare a branch against
#[derive(Serialize, Deserialize, Debug, Clone, Copy, ValueEnum)]
pub enum BranchAgainst {
    /// Compare branch against the current HEAD
    Current,
    /// Compare branch against the repository's `main`
    Main,
}

impl Default for BranchAgainst {
    fn default() -> Self {
        BranchAgainst::Main
    }
}
