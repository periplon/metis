use clap::Parser;
use metis::adapters::encryption;
use metis::adapters::mock_strategy::MockStrategyHandler;
use metis::adapters::prompt_handler::InMemoryPromptHandler;
use metis::adapters::resource_handler::InMemoryResourceHandler;
use metis::adapters::rmcp_server::MetisServer;
use metis::adapters::secrets::{create_passphrase_store, create_secrets_store, keys};
use metis::adapters::state_manager::StateManager;
use metis::adapters::tool_handler::BasicToolHandler;
use metis::cli::{Cli, Commands};
use metis::config::{watcher::ConfigWatcher, s3_watcher::AwsCredentials, S3Watcher, Settings};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle subcommands first (these don't need the full server setup)
    if let Some(cmd) = &cli.command {
        return handle_command(cmd);
    }

    // Initialize tracing (only for server mode)
    tracing_subscriber::fmt::init();

    // Load configuration with CLI overrides
    let settings = Settings::new_with_cli(&cli)?;
    let host = settings.server.host.clone();
    let port = settings.server.port;
    let s3_config = settings.s3.clone();

    info!("Starting Metis MCP Mock Server on {}:{}", host, port);

    // Wrap settings in Arc<RwLock> for live reload
    let settings = Arc::new(RwLock::new(settings));

    // Start config watcher for local file changes
    let settings_for_watcher = settings.clone();
    let paths = vec![
        "metis.toml".to_string(),
        "config/tools".to_string(),
        "config/resources".to_string(),
        "config/resource_templates".to_string(),
        "config/prompts".to_string(),
    ];
    let _watcher = ConfigWatcher::new(paths, move || {
        match Settings::new() {
            Ok(new_settings) => {
                let mut w = settings_for_watcher.blocking_write();
                // Merge local config changes (local config is base, gets overridden by S3/UI)
                w.merge(new_settings);
                info!("Configuration merged from local files successfully");
            }
            Err(e) => error!("Failed to reload configuration: {}", e),
        }
    })?;

    // Start S3 watcher if enabled AND credentials are available
    let _s3_watcher = if let Some(ref s3_cfg) = s3_config {
        if s3_cfg.is_active() {
            // Get AWS credentials from multiple sources with precedence:
            // 1. Config file (may be encrypted) - higher priority
            // 2. Environment variables - fallback
            let passphrase = cli.secret_passphrase.as_deref();
            let settings_read = settings.read().await;

            // Try to get credentials from config file first (decrypt if needed)
            let config_credentials: Option<AwsCredentials> = {
                let access_key = settings_read.secrets.aws_access_key_id.as_ref();
                let secret_key = settings_read.secrets.aws_secret_access_key.as_ref();

                match (access_key, secret_key) {
                    (Some(ak), Some(sk)) => {
                        // Decrypt if encrypted
                        let decrypted_ak = encryption::decrypt_if_encrypted(ak, passphrase).ok();
                        let decrypted_sk = encryption::decrypt_if_encrypted(sk, passphrase).ok();

                        match (decrypted_ak, decrypted_sk) {
                            (Some(ak_val), Some(sk_val)) => Some(AwsCredentials {
                                access_key_id: ak_val,
                                secret_access_key: sk_val,
                            }),
                            _ => None
                        }
                    }
                    _ => None
                }
            };
            drop(settings_read);

            // Fall back to environment variables if no config credentials
            let env_credentials: Option<AwsCredentials> = {
                match (std::env::var("AWS_ACCESS_KEY_ID"), std::env::var("AWS_SECRET_ACCESS_KEY")) {
                    (Ok(ak), Ok(sk)) => Some(AwsCredentials {
                        access_key_id: ak,
                        secret_access_key: sk,
                    }),
                    _ => None
                }
            };

            // Use config credentials first, then env credentials
            let credentials = config_credentials.or(env_credentials);

            if credentials.is_some() {
                info!(
                    "Starting S3 configuration watcher for bucket: {}",
                    s3_cfg.bucket.as_ref().unwrap_or(&"unknown".to_string())
                );
                match S3Watcher::new_with_credentials(s3_cfg, credentials).await {
                    Ok(s3_watcher) => {
                        let settings_for_s3 = settings.clone();
                        if let Err(e) = s3_watcher
                            .start_with_callback(move |s3_configs| {
                                let settings_clone = settings_for_s3.clone();
                                // Spawn a new task to handle the async settings update
                                tokio::spawn(async move {
                                    let mut w = settings_clone.write().await;
                                    // Merge S3 configs into existing settings (S3 takes precedence)
                                    w.merge_s3_configs(s3_configs);
                                    info!("Configuration merged from S3 successfully");
                                });
                            })
                            .await
                        {
                            warn!("Failed to start S3 watcher: {}", e);
                        }
                        Some(s3_watcher)
                    }
                    Err(e) => {
                        warn!("Failed to initialize S3 watcher: {}", e);
                        None
                    }
                }
            } else {
                info!(
                    "S3 is enabled but AWS credentials not found. \
                    S3 watcher will not start. Provide credentials via: \
                    environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY), \
                    config file [secrets] section, or UI Secrets section."
                );
                None
            }
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

    // Create in-memory secrets store for API keys
    let secrets_store = create_secrets_store();
    info!("Initialized in-memory secrets store");

    // Create passphrase store and populate if passphrase is provided
    let passphrase_store = create_passphrase_store();
    if let Some(passphrase) = &cli.secret_passphrase {
        passphrase_store.set(passphrase).await;
        info!("Passphrase configured for encrypting secrets when saving config");
    }

    // Load secrets from config file into secrets store
    {
        let settings_read = settings.read().await;
        let passphrase = cli.secret_passphrase.as_deref();
        load_secrets_from_config(&settings_read.secrets, &secrets_store, passphrase).await;
    }

    // Create agent handler with secrets support
    let agent_handler = metis::agents::handler::AgentHandler::new_with_secrets(
        settings.clone(),
        tool_handler.clone(),
        secrets_store.clone(),
    );

    // Initialize agents
    if let Err(e) = agent_handler.initialize().await {
        tracing::warn!("Failed to initialize agents: {}", e);
    } else {
        info!("Agents initialized successfully");
    }

    // Wrap in Arc for sharing
    let agent_handler: Arc<dyn metis::agents::domain::AgentPort> = Arc::new(agent_handler);

    // Wire up agent handler to tool handler so agents can call other agents
    // (tool_handler handles agent tools, MCP tools, workflows, and regular tools)
    tool_handler.set_agent_handler(agent_handler).await;

    // Create MetisServer (tool_handler already includes agent support)
    let metis_server = MetisServer::new(
        resource_handler,
        tool_handler.clone(),
        prompt_handler,
    );

    // Create application using the library function
    let app = metis::create_app(metis_server, health_handler, metrics_handler, settings, state_manager, secrets_store, passphrase_store, tool_handler).await;

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle CLI subcommands
fn handle_command(cmd: &Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::EncryptSecret { value, passphrase } => {
            let pass = get_passphrase(passphrase.as_deref(), "Enter passphrase for encryption: ")?;
            match encryption::encrypt(value, &pass) {
                Ok(encrypted) => {
                    println!("{}", encrypted);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Encryption failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::DecryptSecret { value, passphrase } => {
            let pass = get_passphrase(passphrase.as_deref(), "Enter passphrase for decryption: ")?;
            match encryption::decrypt(value, &pass) {
                Ok(decrypted) => {
                    println!("{}", decrypted);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Decryption failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Get passphrase from argument or prompt user
fn get_passphrase(provided: Option<&str>, prompt: &str) -> anyhow::Result<String> {
    if let Some(pass) = provided {
        return Ok(pass.to_string());
    }

    // Prompt for passphrase
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut passphrase = String::new();
    io::stdin().read_line(&mut passphrase)?;

    Ok(passphrase.trim().to_string())
}

/// Load secrets from config into the secrets store
async fn load_secrets_from_config(
    secrets_config: &metis::config::SecretsConfig,
    secrets_store: &metis::adapters::secrets::SharedSecretsStore,
    passphrase: Option<&str>,
) {
    let mut loaded_count = 0;

    // Helper to decrypt and load a secret
    let mut load_secret = |key: &str, value: Option<&String>| {
        if let Some(val) = value {
            match encryption::decrypt_if_encrypted(val, passphrase) {
                Ok(decrypted) => {
                    // Use blocking runtime context since we're in async
                    let store = secrets_store.clone();
                    let k = key.to_string();
                    let v = decrypted;
                    tokio::spawn(async move {
                        store.set(&k, &v).await;
                    });
                    loaded_count += 1;
                    info!("Loaded secret {} from config", key);
                }
                Err(e) => {
                    if encryption::is_encrypted(val) {
                        warn!("Failed to decrypt secret {}: {} (passphrase may be missing or incorrect)", key, e);
                    } else {
                        // Plain text value, load it
                        let store = secrets_store.clone();
                        let k = key.to_string();
                        let v = val.clone();
                        tokio::spawn(async move {
                            store.set(&k, &v).await;
                        });
                        loaded_count += 1;
                        info!("Loaded secret {} from config", key);
                    }
                }
            }
        }
    };

    load_secret(keys::OPENAI_API_KEY, secrets_config.openai_api_key.as_ref());
    load_secret(keys::ANTHROPIC_API_KEY, secrets_config.anthropic_api_key.as_ref());
    load_secret(keys::GEMINI_API_KEY, secrets_config.gemini_api_key.as_ref());
    load_secret(keys::AWS_ACCESS_KEY_ID, secrets_config.aws_access_key_id.as_ref());
    load_secret(keys::AWS_SECRET_ACCESS_KEY, secrets_config.aws_secret_access_key.as_ref());
    load_secret(keys::AWS_REGION, secrets_config.aws_region.as_ref());

    if loaded_count > 0 {
        info!("Loaded {} secrets from config file", loaded_count);
    }
}
