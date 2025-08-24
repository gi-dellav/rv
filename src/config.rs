use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DiffProfile {
    report_diffs: bool,
    report_sources: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum CustomPromptMode {
    #[default]
    None,
    
    Replace,
    Suffix,
    Prefix,
}

#[derive(Serialize, Deserialize, Debug)]
/// LLM provider specific configuration
pub struct LLMConfig {
    configuration_name: String,
    provider_id: String,
    model_id: String,
    api_key: String,

    custom_prompt_mode: CustomPromptMode,
    custom_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Main configuration structure, used in `~/.config/rv/config.toml`
pub struct RvConfig {
    diff_profile: DiffProfile,
    llm_configs: Vec<LLMConfig>,
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
            provider_id: String::from("openrouter"),
            model_id: String::from("chatgpt-4o-mini"),
            api_key: String::from("[insert api key here]"),
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
        }
    }
}
