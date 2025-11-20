use config::{Config, File};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod watcher;
pub mod validator;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    #[serde(default)]
    pub auth: crate::domain::auth::AuthConfig,
    #[serde(default)]
    pub resources: Vec<ResourceConfig>,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    #[serde(default)]
    pub prompts: Vec<PromptConfig>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MockStrategyType {
    Static,
    Template,
    Random,
    Stateful,
    Script,
    File,
    Pattern,
    #[serde(rename = "llm")]
    LLM,
    #[serde(rename = "database")]
    Database,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MockConfig {
    pub strategy: MockStrategyType,
    pub template: Option<String>,
    pub faker_type: Option<String>,
    pub stateful: Option<StatefulConfig>,
    pub script: Option<String>,
    #[serde(default)]
    pub script_lang: Option<ScriptLang>,
    pub file: Option<FileConfig>,
    pub pattern: Option<String>,
    pub llm: Option<LLMConfig>,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    Rhai,
    Lua,
    Js,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub query: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    #[serde(default)]
    pub api_key_env: Option<String>,
    pub model: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    pub path: String,
    #[serde(default = "default_selection")]
    pub selection: String, // "random", "sequential", "weighted"
}

fn default_selection() -> String {
    "random".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatefulConfig {
    pub state_key: String,
    pub operation: StateOperation,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub fn new() -> Result<Self, anyhow::Error> {
        Self::from_root(".")
    }

    pub fn from_root(root: &str) -> Result<Self, anyhow::Error> {
        let config_path = std::path::Path::new(root).join("metis");
        let s = Config::builder()
            .add_source(File::from(config_path).required(false))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;

        settings.load_external_configs(root)?;

        // Validate configuration
        validator::ConfigValidator::validate(&settings)
            .map_err(|errors| {
                let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                anyhow::anyhow!("Configuration validation failed:\n{}", error_messages.join("\n"))
            })?;

        Ok(settings)
    }

    fn load_external_configs(&mut self, root: &str) -> Result<(), anyhow::Error> {
        self.load_tools_from_dir(&format!("{}/config/tools", root))?;
        self.load_resources_from_dir(&format!("{}/config/resources", root))?;
        self.load_prompts_from_dir(&format!("{}/config/prompts", root))?;
        Ok(())
    }

    fn load_tools_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let tool: ToolConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.tools.push(tool);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_resources_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let resource: ResourceConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.resources.push(resource);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_prompts_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let prompt: PromptConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.prompts.push(prompt);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }
}
