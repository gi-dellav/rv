use anyhow::Result;

pub trait LLMProvider {
    fn get_provider_name(self) -> String;

    fn set_api_key(self, key: String) -> Result<()>;
    fn set_model(self, model: String) -> Result<()>;
    //fn set_timeout(self, timeout_sec: i32) -> Result<()>;

    fn stream_request_stdout(self, sys_prompt: String, review_prompt: String);
}
