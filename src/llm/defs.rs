use anyhow::Result;
use rig::message::Message;

pub trait LLMProvider {
    fn get_provider_name(&self) -> String;
    fn stream_request_stdout(&self, sys_prompt: String, messages: Vec<Message>) -> Result<String>;
}
