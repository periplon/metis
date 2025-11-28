use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use super::s3::S3Config;

/// Tracks the state of S3 objects for change detection
#[derive(Debug, Clone, Default)]
struct ObjectState {
    etag: Option<String>,
    last_modified: Option<String>,
}

/// S3 configuration watcher that polls for changes and triggers reloads
pub struct S3Watcher {
    client: S3Client,
    config: S3Config,
    object_states: Arc<RwLock<HashMap<String, ObjectState>>>,
    running: Arc<RwLock<bool>>,
}

/// AWS credentials for S3 access
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl S3Watcher {
    /// Create a new S3Watcher with the given configuration
    pub async fn new(config: &S3Config) -> anyhow::Result<Self> {
        Self::new_with_credentials(config, None).await
    }

    /// Create a new S3Watcher with explicit credentials
    pub async fn new_with_credentials(
        config: &S3Config,
        credentials: Option<AwsCredentials>,
    ) -> anyhow::Result<Self> {
        let sdk_config = Self::build_aws_config(config, credentials).await?;
        let client = S3Client::new(&sdk_config);

        Ok(Self {
            client,
            config: config.clone(),
            object_states: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Build AWS SDK configuration with optional custom endpoint and credentials
    async fn build_aws_config(
        config: &S3Config,
        credentials: Option<AwsCredentials>,
    ) -> anyhow::Result<aws_config::SdkConfig> {
        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = &config.region {
            loader = loader.region(aws_config::Region::new(region.clone()));
        }

        // If custom endpoint is specified
        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        // If explicit credentials are provided, use them
        if let Some(creds) = credentials {
            let aws_creds = aws_sdk_s3::config::Credentials::new(
                creds.access_key_id,
                creds.secret_access_key,
                None, // session token
                None, // expiry
                "metis-s3-watcher",
            );
            loader = loader.credentials_provider(aws_creds);
            debug!("S3 watcher using explicit credentials");
        } else {
            debug!("S3 watcher using default credential chain");
        }

        Ok(loader.load().await)
    }

    /// Start watching for configuration changes (legacy - doesn't fetch S3 content)
    pub async fn start<F>(&self, on_change: F) -> anyhow::Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.start_with_callback(move |_configs| {
            on_change();
        }).await
    }

    /// Start watching for configuration changes with access to fetched S3 configs
    /// The callback receives a Vec of (key, content) tuples for all config files in S3
    pub async fn start_with_callback<F>(&self, on_change: F) -> anyhow::Result<()>
    where
        F: Fn(Vec<(String, String)>) + Send + Sync + 'static,
    {
        let bucket = self.config.bucket.as_ref().ok_or_else(|| {
            anyhow::anyhow!("S3 bucket is required")
        })?;

        {
            let mut running = self.running.write().await;
            if *running {
                return Err(anyhow::anyhow!("S3 watcher is already running"));
            }
            *running = true;
        }

        info!(
            "Starting S3 configuration watcher for bucket: {} with prefix: {}",
            bucket,
            self.config.get_prefix()
        );

        let client = self.client.clone();
        let config = self.config.clone();
        let object_states = self.object_states.clone();
        let running = self.running.clone();
        let on_change = Arc::new(on_change);

        // Do initial fetch on startup
        match Self::fetch_all_configs(&client, &config).await {
            Ok(configs) => {
                if !configs.is_empty() {
                    info!("Initial S3 config fetch: {} files loaded", configs.len());
                    on_change(configs);
                }
            }
            Err(e) => {
                warn!("Failed to fetch initial S3 configs: {}", e);
            }
        }

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.poll_interval_secs));

            loop {
                interval.tick().await;

                let is_running = *running.read().await;
                if !is_running {
                    info!("S3 watcher stopped");
                    break;
                }

                match Self::check_for_changes(&client, &config, &object_states).await {
                    Ok(changed) => {
                        if changed {
                            info!("S3 configuration changed, fetching updated configs");
                            // Fetch the actual configs from S3
                            match Self::fetch_all_configs(&client, &config).await {
                                Ok(configs) => {
                                    info!("Fetched {} config files from S3", configs.len());
                                    on_change(configs);
                                }
                                Err(e) => {
                                    error!("Failed to fetch S3 configs after change detected: {}", e);
                                }
                            }
                        } else {
                            debug!("No S3 configuration changes detected");
                        }
                    }
                    Err(e) => {
                        // Provide more helpful error messages for common S3 issues
                        let error_str = format!("{:?}", e);
                        if error_str.contains("credentials") || error_str.contains("Credentials") || error_str.contains("NoCredentialsError") {
                            warn!(
                                "S3 watcher: No valid AWS credentials found. \
                                Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables, \
                                or configure credentials in the UI Secrets section. Error: {}",
                                e
                            );
                        } else if error_str.contains("NoSuchBucket") {
                            error!(
                                "S3 watcher: Bucket '{}' does not exist or you don't have access. Error: {}",
                                config.bucket.as_deref().unwrap_or("unknown"),
                                e
                            );
                        } else if error_str.contains("AccessDenied") || error_str.contains("Forbidden") {
                            error!(
                                "S3 watcher: Access denied to bucket '{}'. Check credentials and bucket permissions. Error: {}",
                                config.bucket.as_deref().unwrap_or("unknown"),
                                e
                            );
                        } else {
                            error!(
                                "S3 watcher error checking for changes: {}. Debug: {:?}",
                                e, e
                            );
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Fetch all config files from S3 (static method for use in spawned task)
    async fn fetch_all_configs(
        client: &S3Client,
        config: &S3Config,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let bucket = config.bucket.as_ref().ok_or_else(|| {
            anyhow::anyhow!("S3 bucket is required")
        })?;
        let prefix = config.get_prefix();

        let list_result = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let mut configs = Vec::new();

        for object in list_result.contents() {
            let key = match object.key() {
                Some(k) if Self::is_config_file(k) => k,
                _ => continue,
            };

            match client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
            {
                Ok(result) => {
                    match result.body.collect().await {
                        Ok(body) => {
                            match String::from_utf8(body.into_bytes().to_vec()) {
                                Ok(content) => {
                                    configs.push((key.to_string(), content));
                                }
                                Err(e) => {
                                    warn!("S3 object {} contains invalid UTF-8: {}", key, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read S3 object body {}: {}", key, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch S3 object {}: {}", key, e);
                }
            }
        }

        Ok(configs)
    }

    /// Stop watching for changes
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopping S3 watcher");
    }

    /// Check for changes in S3 objects
    async fn check_for_changes(
        client: &S3Client,
        config: &S3Config,
        object_states: &Arc<RwLock<HashMap<String, ObjectState>>>,
    ) -> anyhow::Result<bool> {
        let bucket = config.bucket.as_ref().ok_or_else(|| {
            anyhow::anyhow!("S3 bucket is required")
        })?;
        let prefix = config.get_prefix();

        // List objects with the configured prefix
        let list_result = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let objects = list_result.contents();
        let mut states = object_states.write().await;
        let mut changed = false;

        // Check each object for changes
        for object in objects {
            let key = object.key().unwrap_or_default();

            // Only watch YAML, JSON, and TOML files
            if !Self::is_config_file(key) {
                continue;
            }

            let current_state = ObjectState {
                etag: object.e_tag().map(|s| s.to_string()),
                last_modified: object.last_modified().map(|t| t.to_string()),
            };

            if let Some(previous_state) = states.get(key) {
                if previous_state.etag != current_state.etag
                    || previous_state.last_modified != current_state.last_modified
                {
                    info!("S3 object changed: {}", key);
                    changed = true;
                }
            } else {
                // New object
                info!("New S3 object detected: {}", key);
                changed = true;
            }

            states.insert(key.to_string(), current_state);
        }

        // Check for deleted objects
        let current_keys: std::collections::HashSet<_> = objects
            .iter()
            .filter_map(|o| o.key())
            .filter(|k| Self::is_config_file(k))
            .map(|k| k.to_string())
            .collect();

        let removed_keys: Vec<_> = states
            .keys()
            .filter(|k| !current_keys.contains(*k))
            .cloned()
            .collect();

        for key in removed_keys {
            info!("S3 object removed: {}", key);
            states.remove(&key);
            changed = true;
        }

        Ok(changed)
    }

    /// Check if a file is a configuration file based on extension
    fn is_config_file(key: &str) -> bool {
        key.ends_with(".yaml")
            || key.ends_with(".yml")
            || key.ends_with(".json")
            || key.ends_with(".toml")
    }

    /// Fetch configuration files from S3
    pub async fn fetch_configs(&self) -> anyhow::Result<Vec<(String, String)>> {
        let bucket = self.config.bucket.as_ref().ok_or_else(|| {
            anyhow::anyhow!("S3 bucket is required")
        })?;
        let prefix = self.config.get_prefix();

        let list_result = self
            .client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(&prefix)
            .send()
            .await?;

        let mut configs = Vec::new();

        for object in list_result.contents() {
            let key = match object.key() {
                Some(k) if Self::is_config_file(k) => k,
                _ => continue,
            };

            match self.fetch_object(bucket, key).await {
                Ok(content) => {
                    configs.push((key.to_string(), content));
                }
                Err(e) => {
                    warn!("Failed to fetch S3 object {}: {}", key, e);
                }
            }
        }

        Ok(configs)
    }

    /// Fetch a single object from S3
    async fn fetch_object(&self, bucket: &str, key: &str) -> anyhow::Result<String> {
        let result = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;

        let body = result.body.collect().await?;
        let content = String::from_utf8(body.into_bytes().to_vec())?;

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_config_file() {
        assert!(S3Watcher::is_config_file("config/tools/test.yaml"));
        assert!(S3Watcher::is_config_file("config/tools/test.yml"));
        assert!(S3Watcher::is_config_file("config/resources/test.json"));
        assert!(S3Watcher::is_config_file("metis.toml"));
        assert!(!S3Watcher::is_config_file("config/test.txt"));
        assert!(!S3Watcher::is_config_file("config/test.md"));
    }
}
