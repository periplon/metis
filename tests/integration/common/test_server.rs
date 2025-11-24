use metis::adapters::{
    health_handler::HealthHandler,
    metrics_handler::{MetricsCollector, MetricsHandler},
    mock_strategy::MockStrategyHandler,
    prompt_handler::InMemoryPromptHandler,
    resource_handler::InMemoryResourceHandler,
    rmcp_server::MetisServer,
    state_manager::StateManager,
    tool_handler::BasicToolHandler,
};
use metis::config::Settings;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

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
            rate_limit: None,
        }));

        // Initialize handlers
        let state_manager = Arc::new(StateManager::new());
        let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));
        let resource_handler = Arc::new(InMemoryResourceHandler::new(
            settings.clone(),
            mock_strategy.clone(),
        ));
        let tool_handler = Arc::new(BasicToolHandler::new(
            settings.clone(),
            mock_strategy.clone(),
        ));
        let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
        let health_handler = Arc::new(HealthHandler::new(settings.clone()));
        let metrics_collector = Arc::new(MetricsCollector::new().unwrap());
        let metrics_handler = Arc::new(MetricsHandler::new(metrics_collector));

        // Create MetisServer using rmcp SDK
        let metis_server = MetisServer::new(resource_handler, tool_handler, prompt_handler);

        // Create app
        let app =
            metis::create_app(metis_server, health_handler, metrics_handler, settings).await;

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
