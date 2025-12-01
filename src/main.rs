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
use metis::persistence::DataStore;
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
        return handle_command(cmd, &cli).await;
    }

    // Initialize tracing (only for server mode)
    tracing_subscriber::fmt::init();

    // Load configuration with CLI overrides
    let settings = Settings::new_with_cli(&cli)?;
    let host = settings.server.host.clone();
    let port = settings.server.port;
    let s3_config = settings.s3.clone();

    // Get the config root directory for watching and reloading
    let config_path = settings
        .config_path
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("metis.toml"));
    let config_root = config_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let config_root_for_reload = config_root.clone();

    info!("Starting Metis MCP Mock Server on {}:{}", host, port);
    info!("Using configuration file: {}", config_path.display());

    // Wrap settings in Arc<RwLock> for live reload
    let settings = Arc::new(RwLock::new(settings));

    // Start config watcher for local file changes
    let settings_for_watcher = settings.clone();
    // Build watch paths based on the actual config location
    let paths = vec![
        config_path.to_string_lossy().to_string(),
        format!("{}/config/tools", config_root),
        format!("{}/config/resources", config_root),
        format!("{}/config/resource_templates", config_root),
        format!("{}/config/prompts", config_root),
        format!("{}/config/schemas", config_root),
    ];
    let _watcher = ConfigWatcher::new(paths, move || {
        match Settings::from_root(&config_root_for_reload) {
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

    // Initialize database if configured
    let data_store: Option<Arc<DataStore>> = {
        let settings_read = settings.read().await;
        if let Some(db_config) = &settings_read.database {
            info!("Initializing database connection: {}", db_config.url);
            match DataStore::new(db_config).await {
                Ok(store) => {
                    info!("Database connection established");

                    // Seed from settings if enabled
                    if db_config.seed_on_startup {
                        if let Err(e) = store.seed_from_settings(&settings_read).await {
                            warn!("Failed to seed database from config: {}", e);
                        } else {
                            info!("Database seeded from configuration");
                        }
                    }

                    Some(Arc::new(store))
                }
                Err(e) => {
                    error!("Failed to initialize database: {}", e);
                    None
                }
            }
        } else {
            info!("No database configured, running in config-only mode");
            None
        }
    };

    // Create application using the library function
    let app = metis::create_app(metis_server, health_handler, metrics_handler, settings, state_manager, secrets_store, passphrase_store, tool_handler, data_store).await;

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle CLI subcommands
async fn handle_command(cmd: &Commands, cli: &Cli) -> anyhow::Result<()> {
    use metis::persistence::{ArchetypeRepository, CommitRepository, DataStore, PersistenceConfig};

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
        Commands::Migrate { database_url } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            println!("Running database migrations...");
            let store = DataStore::new(&config).await?;
            let result = store.migrate().await?;
            println!("Migrations completed: {} applied, {} skipped", result.applied, result.skipped);
            Ok(())
        }
        Commands::MigrateStatus { database_url } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            let store = DataStore::new(&config).await?;
            let statuses = store.migration_status().await?;

            println!("Migration Status:");
            println!("{:-<60}", "");
            for status in statuses {
                let applied_marker = if status.applied { "[x]" } else { "[ ]" };
                let applied_at = status.applied_at.as_deref().unwrap_or("-");
                println!("{} {} (applied: {})", applied_marker, status.name, applied_at);
            }
            Ok(())
        }
        Commands::Export {
            output,
            format,
            from_database,
        } => {
            // Load settings
            let settings = Settings::new_with_cli(cli)?;

            let export_data = if *from_database {
                // Export from database
                if let Some(db_config) = &settings.database {
                    let store = DataStore::new(db_config).await?;
                    let archetypes = store.archetypes().export_all().await?;

                    // Export archetypes as a simple JSON object
                    // We can't easily construct Settings without all required fields
                    // Instead, export the archetypes directly
                    let json_data = serde_json::to_value(&archetypes)?;

                    // Return as a pseudo-Settings by wrapping archetypes appropriately
                    let export_value = serde_json::json!({
                        "server": { "host": "0.0.0.0", "port": 8080 },
                        "resources": archetypes.get("resource").unwrap_or(&vec![]),
                        "resource_templates": archetypes.get("resource_template").unwrap_or(&vec![]),
                        "tools": archetypes.get("tool").unwrap_or(&vec![]),
                        "prompts": archetypes.get("prompt").unwrap_or(&vec![]),
                        "workflows": archetypes.get("workflow").unwrap_or(&vec![]),
                        "agents": archetypes.get("agent").unwrap_or(&vec![]),
                        "orchestrations": archetypes.get("orchestration").unwrap_or(&vec![]),
                        "schemas": archetypes.get("schema").unwrap_or(&vec![]),
                    });
                    drop(json_data); // not needed anymore

                    // Output format-specific content and return early
                    let content = match format.to_lowercase().as_str() {
                        "toml" => {
                            // TOML from serde_json value
                            let val: toml::Value = serde_json::from_value(export_value)?;
                            toml::to_string_pretty(&val)?
                        }
                        "json" => serde_json::to_string_pretty(&export_value)?,
                        "yaml" | "yml" => serde_yaml::to_string(&export_value)?,
                        _ => {
                            eprintln!("Unknown format: {}. Use toml, json, or yaml.", format);
                            std::process::exit(1);
                        }
                    };

                    if let Some(path) = output {
                        std::fs::write(path, &content)?;
                        println!("Configuration exported to: {}", path.display());
                    } else {
                        println!("{}", content);
                    }
                    return Ok(());
                } else {
                    eprintln!("Database not configured. Use --from-database only when database is configured.");
                    std::process::exit(1);
                }
            } else {
                settings
            };

            let content = match format.to_lowercase().as_str() {
                "toml" => toml::to_string_pretty(&export_data)?,
                "json" => serde_json::to_string_pretty(&export_data)?,
                "yaml" | "yml" => serde_yaml::to_string(&export_data)?,
                _ => {
                    eprintln!("Unknown format: {}. Use toml, json, or yaml.", format);
                    std::process::exit(1);
                }
            };

            if let Some(path) = output {
                std::fs::write(path, &content)?;
                println!("Configuration exported to: {}", path.display());
            } else {
                println!("{}", content);
            }
            Ok(())
        }
        Commands::Import {
            input,
            format,
            target,
            merge,
        } => {
            // Detect format from extension if not specified
            let fmt = format.clone().unwrap_or_else(|| {
                input
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "toml".to_string())
            });

            let content = std::fs::read_to_string(input)?;
            let import_settings: Settings = match fmt.as_str() {
                "toml" => toml::from_str(&content)?,
                "json" => serde_json::from_str(&content)?,
                "yaml" | "yml" => serde_yaml::from_str(&content)?,
                _ => {
                    eprintln!("Unknown format: {}. Use toml, json, or yaml.", fmt);
                    std::process::exit(1);
                }
            };

            match target.as_str() {
                "database" => {
                    let settings = Settings::new_with_cli(cli)?;
                    if let Some(db_config) = &settings.database {
                        let store = DataStore::new(db_config).await?;

                        if !*merge {
                            // TODO: Clear existing data before import
                            println!("Warning: Replace mode not yet implemented. Using merge mode.");
                        }

                        let count = store.seed_from_settings(&import_settings).await?;
                        println!("Imported {} archetypes to database", count);
                    } else {
                        eprintln!("Database not configured. Configure database in settings first.");
                        std::process::exit(1);
                    }
                }
                "config-file" => {
                    // Export to the config file location
                    let config_path = &cli.config;
                    let content = toml::to_string_pretty(&import_settings)?;
                    std::fs::write(config_path, content)?;
                    println!("Configuration written to: {}", config_path.display());
                }
                _ => {
                    eprintln!("Unknown target: {}. Use 'database' or 'config-file'.", target);
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        Commands::VersionList {
            database_url,
            limit,
            verbose,
        } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            let store = DataStore::new(&config).await?;
            let commits = store.commits().list_commits(*limit, 0).await?;

            if commits.is_empty() {
                println!("No version history found.");
                return Ok(());
            }

            println!("Version History:");
            println!("{:-<80}", "");

            for commit in commits {
                let tag_str = commit.tag.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                let snapshot_str = if commit.is_snapshot { " [snapshot]" } else { "" };
                let author_str = commit.author.as_ref().map(|a| format!(" by {}", a)).unwrap_or_default();

                println!(
                    "{} {} - {}{}{} ({} changes){}",
                    &commit.commit_hash[..8],
                    commit.committed_at,
                    commit.message,
                    author_str,
                    tag_str,
                    commit.changes_count,
                    snapshot_str
                );

                if *verbose {
                    let changesets = store.commits().get_changesets(&commit.id).await?;
                    for cs in changesets {
                        println!("  {} {} {}", cs.operation, cs.archetype_type, cs.archetype_name);
                    }
                    println!();
                }
            }
            Ok(())
        }
        Commands::TagList { database_url } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            let store = DataStore::new(&config).await?;
            let tags = store.commits().list_tags().await?;

            if tags.is_empty() {
                println!("No tags found.");
                return Ok(());
            }

            println!("Tags:");
            println!("{:-<60}", "");

            for tag in tags {
                let msg = tag.message.as_ref().map(|m| format!(" - {}", m)).unwrap_or_default();
                println!("{} -> {}{}  ({})", tag.name, &tag.commit_hash[..8], msg, tag.created_at);
            }
            Ok(())
        }
        Commands::TagCreate {
            name,
            message,
            database_url,
        } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            let store = DataStore::new(&config).await?;

            // Get current HEAD
            let head = store.commits().get_head().await?;
            if let Some(head) = head {
                let tag = store
                    .commits()
                    .create_tag(name, &head.commit_hash, message.as_deref())
                    .await?;
                println!("Created tag '{}' at commit {}", tag.name, &tag.commit_hash[..8]);
            } else {
                eprintln!("No commits found. Create some changes first.");
                std::process::exit(1);
            }
            Ok(())
        }
        Commands::Rollback {
            commit_hash,
            database_url,
        } => {
            let db_url = get_database_url(database_url.as_deref(), cli)?;
            let config = PersistenceConfig {
                url: db_url,
                ..Default::default()
            };

            let store = DataStore::new(&config).await?;

            println!("Rolling back to commit {}...", &commit_hash[..8.min(commit_hash.len())]);
            let rollback_commit = store.commits().rollback_to(commit_hash).await?;
            println!("Rollback completed. Created rollback commit: {}", &rollback_commit.commit_hash[..8]);
            Ok(())
        }
    }
}

/// Get database URL from CLI arg, config, or error
fn get_database_url(cli_url: Option<&str>, cli: &Cli) -> anyhow::Result<String> {
    if let Some(url) = cli_url {
        return Ok(url.to_string());
    }

    // Try to load from config
    match Settings::new_with_cli(cli) {
        Ok(settings) => {
            if let Some(db) = settings.database {
                Ok(db.url)
            } else {
                anyhow::bail!("No database URL provided. Use --database-url or configure [database] in config file.")
            }
        }
        Err(_) => {
            anyhow::bail!("No database URL provided. Use --database-url or configure [database] in config file.")
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
