use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

#[derive(Debug, Deserialize)]
pub struct LogMessage {
    pub level: LogLevel,
    pub logger: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

pub struct LoggingHandler;

impl LoggingHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_log(&self, message: LogMessage) {
        let logger = message.logger.as_deref().unwrap_or("client");
        let data_str = message.data.to_string();

        match message.level {
            LogLevel::Debug => debug!(logger = logger, "{}", data_str),
            LogLevel::Info => info!(logger = logger, "{}", data_str),
            LogLevel::Warning => warn!(logger = logger, "{}", data_str),
            LogLevel::Error => error!(logger = logger, "{}", data_str),
        }
    }
}

impl Default for LoggingHandler {
    fn default() -> Self {
        Self::new()
    }
}
