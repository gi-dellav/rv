use crate::config::LLMConfig;
use crate::llm::defs::LLMProvider;
use anyhow::Result;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::openrouter;
use rig::streaming::StreamingChat;
use rig::message::Message;

pub struct OpenRouterClient {
    pub api_key: String,
    pub model: String,
}

impl OpenRouterClient {
    pub fn from_config(llmconfig: LLMConfig) -> OpenRouterClient {
        OpenRouterClient {
            api_key: llmconfig.api_key,
            model: llmconfig.model_id,
        }
    }

    pub async fn stream_chat(&self, sys_prompt: &str, messages: Vec<Message>) -> Result<String> {
        // Check for OPENROUTER_API_KEY environment variable
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or(self.api_key.clone());

        let client: openrouter::Client = openrouter::Client::new(&api_key)?;

        let model = client.completion_model(&self.model);

        let agent = AgentBuilder::new(model).preamble(sys_prompt).build();

        let mut stream = agent.stream_chat("", messages).await;
        let res = rig::agent::stream_to_stdout(&mut stream).await?;
        let full_text = res.response().to_string();

        Ok(full_text)
    }
}

impl LLMProvider for OpenRouterClient {
    fn get_provider_name(&self) -> String {
        "OpenRouter".to_string()
    }

    fn stream_request_stdout(&self, sys_prompt: String, messages: Vec<Message>) -> Result<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.stream_chat(&sys_prompt, messages))
        })
    }
}
