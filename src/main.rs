use clap::Parser;
use metis::adapters::mock_strategy::MockStrategyHandler;
use metis::adapters::prompt_handler::InMemoryPromptHandler;
use metis::adapters::resource_handler::InMemoryResourceHandler;
use metis::adapters::rmcp_server::MetisServer;
use metis::adapters::state_manager::StateManager;
use metis::adapters::tool_handler::BasicToolHandler;
use metis::cli::Cli;
use metis::config::{watcher::ConfigWatcher, S3Watcher, Settings};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load configuration with CLI overrides
    let settings = Settings::new_with_cli(&cli)?;
    let host = settings.server.host.clone();
    let port = settings.server.port;
    let s3_config = settings.s3.clone();

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

    // Start S3 watcher if enabled
    let _s3_watcher = if let Some(ref s3_cfg) = s3_config {
        if s3_cfg.is_active() {
            info!(
                "Starting S3 configuration watcher for bucket: {}",
                s3_cfg.bucket.as_ref().unwrap_or(&"unknown".to_string())
            );
            let s3_watcher = S3Watcher::new(s3_cfg).await?;
            let settings_for_s3 = settings.clone();
            let cli_clone = cli.clone();
            s3_watcher
                .start(move || {
                    match Settings::new_with_cli(&cli_clone) {
                        Ok(new_settings) => {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                let mut w = settings_for_s3.write().await;
                                *w = new_settings;
                            });
                            info!("Configuration reloaded from S3 successfully");
                        }
                        Err(e) => error!("Failed to reload configuration from S3: {}", e),
                    }
                })
                .await?;
            Some(s3_watcher)
        } else {
            None
        }
    } else {
        None
    };

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new());

    // Initialize mock strategy handler
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager.clone()));

    // Initialize handlers
    let resource_handler = Arc::new(InMemoryResourceHandler::new(
        settings.clone(),
        mock_strategy.clone(),
    ));
    let tool_handler = Arc::new(BasicToolHandler::new(
        settings.clone(),
        mock_strategy.clone(),
    ));
    let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
    let health_handler = Arc::new(metis::adapters::health_handler::HealthHandler::new(
        settings.clone(),
    ));

    // Initialize metrics
    let metrics_collector =
        Arc::new(metis::adapters::metrics_handler::MetricsCollector::new()?);
    let metrics_handler =
        Arc::new(metis::adapters::metrics_handler::MetricsHandler::new(metrics_collector));

    // Create MetisServer using rmcp SDK
    let metis_server = MetisServer::new(resource_handler, tool_handler, prompt_handler);

    // Create application using the library function
    let app = metis::create_app(metis_server, health_handler, metrics_handler, settings, state_manager).await;

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
