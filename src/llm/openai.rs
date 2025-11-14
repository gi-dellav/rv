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
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{Write, stdout};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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
        // Check for OPENROUTER_API_KEY environment variable if provider is OpenRouter
        let api_key = if matches!(self.provider, OpenAIProvider::OpenRouter) {
            std::env::var("OPENROUTER_API_KEY").unwrap_or(self.api_key)
        } else {
            self.api_key
        };

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
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
            .frequency_penalty(0.5)
            .presence_penalty(0.6)
            .stream(true)
            .build()?;

        let mut stream = client.chat().create_stream(request).await?;

        let mut out = stdout();
        let mut full_text = String::new();

        // Create a progress bar
        let pb = ProgressBar::new_spinner();
        pb.set_message("Reasoning...");
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner} {msg}")
                .unwrap(),
        );

        // Start the progress bar
        // Start the progress bar
        let should_stop = Arc::new(AtomicBool::new(false));
        let should_stop_clone = should_stop.clone();
        let pb_clone = pb.clone();

        // Spawn a thread to tick the progress bar until we stop it
        let progress_thread_handle = std::thread::spawn(move || {
            while !should_stop_clone.load(Ordering::Relaxed) {
                pb_clone.tick();
                std::thread::sleep(Duration::from_millis(100));
            }
            pb_clone.finish_and_clear();
        });

        // Use a scope to ensure the progress bar is always cleared
        let result = async {
            let res = async {
                while let Some(item) = stream.next().await {
                    // Mark that we've started receiving tokens - stop the progress bar
                    if !should_stop.load(Ordering::Relaxed) {
                        should_stop.store(true, Ordering::Relaxed);
                    }

                    // Handle potential errors from the stream
                    let chunk = match item {
                        Ok(chunk) => chunk,
                        Err(err) => {
                            // Signal to stop the progress bar before returning error
                            should_stop.store(true, Ordering::Relaxed);
                            return Err(err.into());
                        }
                    };

                    for choice in chunk.choices {
                        if let Some(text) = choice.delta.content {
                            print!("{text}");
                            out.flush()?;
                            full_text.push_str(&text);
                        }
                    }
                }
                Ok(full_text)
            }
            .await;

            // Ensure progress bar is stopped
            should_stop.store(true, Ordering::Relaxed);
            res
        }
        .await;

        // Wait for the progress thread to finish
        let _ = progress_thread_handle.join();

        // Add a newline after stream finishes
        println!();

        result
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
        let res = rt.block_on(self.stream_chat_to_terminal(&sys_prompt, &review_prompt));

        match res {
            Ok(_) => {}
            Err(err) => println!("Failed request to LLM provider: {err:?}"),
        }
    }
}
