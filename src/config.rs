use std::fs::{self, File};
use std::io::Read;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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
            model_id: String::from("openai/gpt-5-mini"),
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
        
        return RvConfig {
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

        return Ok(config);
    }

    pub fn load_default() -> anyhow::Result<RvConfig> {
        let loaded_config: anyhow::Result<RvConfig> = RvConfig::load_from_path(
            String::from("~/.config/rv/config.toml")
        );

        if loaded_config.is_ok() {
            // Return succesfully loaded config
            return Ok(loaded_config.unwrap());
        } else {
            // Create new config
            let new_config: RvConfig = Default::default();

            // Save to disk as config.toml
            let toml_string = toml::to_string_pretty(&new_config)?;
            fs::write("~/.config/rv/config.toml", toml_string)?;

            return Ok(new_config);
        }
    }

    pub fn get_llm_configs(self) -> HashMap<String, LLMConfig> {
        let mut llm_hashmap: HashMap<String, LLMConfig> = HashMap::new();

        for lc in self.llm_configs {
            llm_hashmap.insert(lc.configuration_name.clone(), lc.clone());
        }

        return llm_hashmap
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum OpenAIProvider {
    OpenAI,
    OpenRouter,
}
impl OpenAIProvider {
    pub fn get_endpoint(self) -> String {
        return match self {
            OpenAIProvider::OpenAI => { String::from("https://api.openai.com/v1") },
            OpenAIProvider::OpenRouter => { String::from("https://openrouter.ai/api/v1") },
        };
    } 
}

