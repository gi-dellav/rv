pub mod defs;
pub mod openai;
pub mod openrouter;

use crate::config::{LLMConfig, OpenAIProvider};
use crate::llm::defs::LLMProvider;

pub fn create_llm_provider(config: LLMConfig) -> Box<dyn LLMProvider> {
    match config.provider {
        OpenAIProvider::OpenAI => {
            Box::new(openai::OpenAIClient::from_config(config))
        }
        OpenAIProvider::OpenRouter => {
            Box::new(openrouter::OpenRouterClient::from_config(config))
        }
    }
}
