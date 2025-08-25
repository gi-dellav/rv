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
        return Err(io::Error::other(
            "config path has no parent directory",
        ));
    }

    Ok(path)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct DiffProfile {
    pub report_diffs: bool,
    pub report_sources: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub enum CustomPromptMode {
    #[default]
    None,

    Replace,
    Suffix,
    Prefix,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// LLM provider specific configuration
pub struct LLMConfig {
    pub configuration_name: String,
    pub provider: OpenAIProvider,
    pub model_id: String,
    pub api_key: String,

    pub allow_reasoning: bool,

    pub custom_prompt_mode: CustomPromptMode,
    pub custom_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Main configuration structure, used in `~/.config/rv/config.toml`
pub struct RvConfig {
    pub diff_profile: DiffProfile,
    pub llm_configs: Vec<LLMConfig>,
    pub default_llm_config: String,
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

impl Default for LLMConfig {
    fn default() -> Self {
        LLMConfig {
            configuration_name: String::from("default"),
            provider: OpenAIProvider::OpenRouter,
            model_id: String::from("openai/gpt-4o-mini"),
            api_key: String::from("[insert api key here]"),
            allow_reasoning: true,
            custom_prompt_mode: CustomPromptMode::None,
            custom_prompt: None,
        }
    }
}

impl Default for RvConfig {
    fn default() -> Self {
        let diff_profile: DiffProfile = Default::default();
        let llm_default_config: LLMConfig = Default::default();
        let llm_configs = vec![llm_default_config];

        RvConfig {
            diff_profile,
            llm_configs,
            default_llm_config: String::from("default"),
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
impl OpenAIProvider {
    pub fn get_endpoint(self) -> String {
        match self {
            OpenAIProvider::OpenAI => String::from("https://api.openai.com/v1"),
            OpenAIProvider::OpenRouter => String::from("https://openrouter.ai/api/v1"),
        }
    }
}
