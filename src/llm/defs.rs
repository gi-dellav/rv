use anyhow::Result;

pub trait LLMProvider {
    fn get_provider_name(self) -> String;

    fn set_api_key(self, key: String) -> Result<()>;
    fn set_model(self, model: String) -> Result<()>;
    fn set_timeout(self, timeout_sec: int) -> Result<()>;

    fn execute_request(self, prompt: String) -> String;
}
