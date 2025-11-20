use metis::adapters::logging_handler::LoggingHandler;
use metis::adapters::mcp_protocol_handler::McpProtocolHandler;
use metis::adapters::mock_strategy::MockStrategyHandler;
use metis::adapters::prompt_handler::InMemoryPromptHandler;
use metis::adapters::resource_handler::InMemoryResourceHandler;
use metis::adapters::state_manager::StateManager;
use metis::adapters::tool_handler::BasicToolHandler;
use metis::config::Settings;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = Settings::new()?;
    info!("Starting Metis MCP Mock Server on {}:{}", settings.server.host, settings.server.port);

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new());

    // Initialize mock strategy handler
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));

    // Initialize handlers
    let resource_handler = Arc::new(InMemoryResourceHandler::new(settings.resources, mock_strategy.clone()));
    let tool_handler = Arc::new(BasicToolHandler::new(settings.tools, mock_strategy.clone()));
    let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.prompts));
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
    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
