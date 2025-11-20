use metis::adapters::logging_handler::LoggingHandler;
use metis::adapters::mcp_protocol_handler::McpProtocolHandler;
use metis::adapters::mock_strategy::MockStrategyHandler;
use metis::adapters::prompt_handler::InMemoryPromptHandler;
use metis::adapters::resource_handler::InMemoryResourceHandler;
use metis::adapters::state_manager::StateManager;
use metis::adapters::tool_handler::BasicToolHandler;
use metis::config::{Settings, watcher::ConfigWatcher};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = Settings::new()?;
    let host = settings.server.host.clone();
    let port = settings.server.port;
    
    info!("Starting Metis MCP Mock Server on {}:{}", host, port);

    // Wrap settings in Arc<RwLock> for live reload
    let settings = Arc::new(RwLock::new(settings));
    
    // Start config watcher
    let settings_for_watcher = settings.clone();
    let paths = vec![
        "metis.toml".to_string(),
        "config/tools".to_string(),
        "config/resources".to_string(),
        "config/prompts".to_string(),
    ];
    let _watcher = ConfigWatcher::new(paths, move || {
        match Settings::new() {
            Ok(new_settings) => {
                let mut w = settings_for_watcher.blocking_write();
                *w = new_settings;
                info!("Configuration reloaded successfully");
            }
            Err(e) => error!("Failed to reload configuration: {}", e),
        }
    })?;

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new());

    // Initialize mock strategy handler
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));

    // Initialize handlers
    let resource_handler = Arc::new(InMemoryResourceHandler::new(settings.clone(), mock_strategy.clone()));
    let tool_handler = Arc::new(BasicToolHandler::new(settings.clone(), mock_strategy.clone()));
    let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
    let logging_handler = Arc::new(LoggingHandler::new());
    let protocol_handler = Arc::new(McpProtocolHandler::new(
        resource_handler,
        tool_handler,
        prompt_handler,
        logging_handler,
    ));

    // Create application using the library function
    let app = metis::create_app(protocol_handler);

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
