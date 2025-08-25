use crate::config::{LLMConfig, OpenAIProvider};
use crate::llm::defs::LLMProvider;
use anyhow::Result;
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
    },
};
use futures::StreamExt;
use std::io::{Write, stdout};

pub struct OpenAIClient {
    pub provider: OpenAIProvider,
    pub api_key: String,
    pub model: String,
}

impl OpenAIClient {
    pub fn from_config(llmconfig: LLMConfig) -> OpenAIClient {
        OpenAIClient {
            provider: llmconfig.provider,
            api_key: llmconfig.api_key,
            model: llmconfig.model_id,
        }
    }

    pub async fn stream_chat_to_terminal(
        self,
        sys_prompt: &str,
        review_prompt: &str,
    ) -> Result<String> {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(self.api_key)
            .with_api_base(self.provider.get_endpoint());
        let client = Client::with_config(config);

        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model)
            .messages(vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage::from(
                    sys_prompt,
                )),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage::from(
                    review_prompt,
                )),
            ])
            .temperature(0.0)
            .frequency_penalty(0.0)
            .presence_penalty(0.0)
            .stream(true)
            .build()?;

        let mut stream = client.chat().create_stream(request).await?;

        let mut out = stdout();
        let mut full_text = String::new();

        // Pull chunks from the stream
        while let Some(item) = stream.next().await {
            // item is Result<CreateChatCompletionStreamResponse, OpenAIError>
            let chunk = item?; // propagate errors via anyhow

            for choice in chunk.choices {
                if let Some(text) = choice.delta.content {
                    print!("{text}");
                    out.flush()?;

                    full_text.push_str(&text);
                }
            }
        }

        // newline after stream finishes
        println!();

        Ok(full_text)
    }
}

impl LLMProvider for OpenAIClient {
    fn get_provider_name(self) -> String {
        format!("{:?}", self.provider)
    }

    fn set_api_key(mut self, key: String) -> Result<()> {
        self.api_key = key;
        Ok(())
    }
    fn set_model(mut self, model: String) -> Result<()> {
        self.model = model;
        Ok(())
    }

    fn stream_request_stdout(self, sys_prompt: String, review_prompt: String) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(self.stream_chat_to_terminal(&sys_prompt, &review_prompt));
    }
}
