use metis::config::Settings;
use metis::adapters::{
    mcp_protocol_handler::McpProtocolHandler,
    health_handler::HealthHandler,
    metrics_handler::{MetricsCollector, MetricsHandler},
    resource_handler::InMemoryResourceHandler,
    tool_handler::BasicToolHandler,
    prompt_handler::InMemoryPromptHandler,
    logging_handler::LoggingHandler,
    mock_strategy::MockStrategyHandler,
    state_manager::StateManager,
};
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::SocketAddr;

pub struct TestServer {
    pub addr: SocketAddr,
    pub base_url: String,
}

impl TestServer {
    pub async fn new() -> Self {
        // Create test configuration
        let settings = Arc::new(RwLock::new(Settings {
            server: metis::config::ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 0, // Random port
            },
            auth: Default::default(),
            resources: vec![],
            tools: vec![],
            prompts: vec![],
        }));

        // Initialize handlers
        let state_manager = Arc::new(StateManager::new());
        let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));
        let resource_handler = Arc::new(InMemoryResourceHandler::new(settings.clone(), mock_strategy.clone()));
        let tool_handler = Arc::new(BasicToolHandler::new(settings.clone(), mock_strategy.clone()));
        let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
        let logging_handler = Arc::new(LoggingHandler::new());
        let health_handler = Arc::new(HealthHandler::new(settings.clone()));
        let metrics_collector = Arc::new(MetricsCollector::new().unwrap());
        let metrics_handler = Arc::new(MetricsHandler::new(metrics_collector));

        let protocol_handler = Arc::new(McpProtocolHandler::new(
            resource_handler,
            tool_handler,
            prompt_handler,
            logging_handler,
        ));

        // Create app
        let app = metis::create_app(protocol_handler, health_handler, metrics_handler);

        // Start server on random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        TestServer { addr, base_url }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}
