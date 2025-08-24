use anyhow::Result;
use async_openai::{
    Client,
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestMessageArgs, Role},
};
use futures::StreamExt;
use std::io::{stdout, Write};
use crate::config::OpenAIProvider;
use crate::llm::defs::LLMProvider;

pub struct OpenAIClient {
    pub provider: OpenAIProvider,
    pub api_key: String,
    pub model: String,
}

impl OpenAIClient {

pub async fn stream_chat_to_terminal(self, prompt: &str) -> Result<String> {
    let config = async_openai::config::OpenAIConfig::new()
        .with_api_key(self.api_key)
        .with_api_base(self.provider.get_endpoint());
    let client = Client::with_config(config);

    let user_msg = ChatCompletionRequestMessageArgs::default()
        .role(Role::User)
        .content(prompt)
        .build()?;

    let request = CreateChatCompletionRequestArgs::default()
        .model(self.model)
        .messages([user_msg])
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
                print!("{}", text);
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
        return fmt!("{:?}", self.provider);
    }

    fn set_api_key(self, key: String) -> Result<()> {
        self.api_key = key;
        return Ok();
    }
    fn set_model(self, model: String) -> Result<()> {
        self.model = model;
        return Ok();
    }
    
    fn stream_request_stdout(self, prompt: String) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.stream_chat_to_terminal(&prompt))
    }
}
