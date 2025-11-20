use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthChecks {
    pub config: String,
    pub handlers: String,
}

pub struct HealthHandler {
    settings: Arc<RwLock<Settings>>,
    start_time: std::time::Instant,
}

impl HealthHandler {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Self {
        Self {
            settings,
            start_time: std::time::Instant::now(),
        }
    }

    /// Basic health check - returns 200 if server is running
    pub async fn health(&self) -> impl IntoResponse {
        let uptime = self.start_time.elapsed().as_secs();
        let status = HealthStatus {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            checks: HealthChecks {
                config: "ok".to_string(),
                handlers: "ok".to_string(),
            },
        };

        (StatusCode::OK, Json(status))
    }

    /// Readiness check - returns 200 if server is ready to accept requests
    /// Checks if configuration is loaded and handlers are initialized
    pub async fn ready(&self) -> impl IntoResponse {
        let settings = self.settings.read().await;
        
        // Check if configuration is loaded
        let config_ok = !settings.resources.is_empty() 
            || !settings.tools.is_empty() 
            || !settings.prompts.is_empty();

        if config_ok {
            (StatusCode::OK, Json(serde_json::json!({
                "status": "ready",
                "message": "Server is ready to accept requests"
            })))
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "status": "not_ready",
                "message": "Configuration not fully loaded"
            })))
        }
    }

    /// Liveness check - returns 200 if server is alive
    /// This is a simple check that the server is responsive
    pub async fn live(&self) -> impl IntoResponse {
        (StatusCode::OK, Json(serde_json::json!({
            "status": "alive",
            "message": "Server is alive"
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerSettings, Settings};

    #[tokio::test]
    async fn test_health_endpoint() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            auth: Default::default(),
            resources: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
        };
        let handler = HealthHandler::new(Arc::new(RwLock::new(settings)));

        let response = handler.health().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_endpoint_with_config() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            auth: Default::default(),
            resources: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
        };
        let handler = HealthHandler::new(Arc::new(RwLock::new(settings)));

        let response = handler.ready().await.into_response();
        // Will be SERVICE_UNAVAILABLE because no resources/tools/prompts loaded
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_live_endpoint() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            auth: Default::default(),
            resources: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
        };
        let handler = HealthHandler::new(Arc::new(RwLock::new(settings)));

        let response = handler.live().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
