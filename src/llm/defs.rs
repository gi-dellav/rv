use anyhow::Result;

pub trait LLMProvider {
    fn get_provider_name(&self) -> String;
    fn stream_request_stdout(&self, sys_prompt: String, review_prompt: String) -> Result<String>;
}
