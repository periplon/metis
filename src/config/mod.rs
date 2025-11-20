use config::{Config, ConfigError, File};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    #[serde(default)]
    pub resources: Vec<ResourceConfig>,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    #[serde(default)]
    pub prompts: Vec<PromptConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResourceConfig {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub content: Option<String>, // Simple static content for now
    pub mock: Option<MockConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub static_response: Option<Value>, // Simple static response for now
    pub mock: Option<MockConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MockConfig {
    pub strategy: MockStrategyType,
    pub template: Option<String>,
    pub faker_type: Option<String>,
    pub stateful: Option<StatefulConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MockStrategyType {
    Static,
    Template,
    Random,
    Stateful,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StatefulConfig {
    pub state_key: String,
    pub operation: StateOperation,
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StateOperation {
    Get,
    Set,
    Increment,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PromptConfig {
    pub name: String,
    pub description: String,
    pub arguments: Option<Vec<PromptArgument>>,
    pub messages: Option<Vec<PromptMessage>>, // Static messages for now
}

#[derive(Debug, Deserialize, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("metis").required(false))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            .build()?;

        s.try_deserialize()
    }
}
