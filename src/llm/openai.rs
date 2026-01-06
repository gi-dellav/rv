use crate::config::LLMConfig;
use crate::llm::defs::LLMProvider;
use anyhow::Result;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::openai;
use rig::streaming::StreamingChat;
use rig::message::Message;

pub struct OpenAIClient {
    pub api_key: String,
    pub model: String,
}

impl OpenAIClient {
    pub fn from_config(llmconfig: LLMConfig) -> OpenAIClient {
        OpenAIClient {
            api_key: llmconfig.api_key,
            model: llmconfig.model_id,
        }
    }

    pub async fn stream_chat(&self, sys_prompt: &str, messages: Vec<Message>) -> Result<String> {
        let client: openai::Client = openai::Client::new(&self.api_key)?;

        let model = client.completion_model(&self.model);

        let agent = AgentBuilder::new(model).preamble(sys_prompt).build();

        let mut stream = agent.stream_chat("", messages).await;
        let res = rig::agent::stream_to_stdout(&mut stream).await?;
        let full_text = res.response().to_string();

        Ok(full_text)
    }
}

impl LLMProvider for OpenAIClient {
    fn get_provider_name(&self) -> String {
        "OpenAI".to_string()
    }

    fn stream_request_stdout(&self, sys_prompt: String, messages: Vec<Message>) -> Result<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.stream_chat(&sys_prompt, messages))
        })
    }
}
