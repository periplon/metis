use serde::Deserialize;

use crate::cli::Cli;

/// S3 configuration for remote configuration source
#[derive(Debug, Deserialize, Clone)]
pub struct S3Config {
    /// Whether S3 configuration source is enabled
    #[serde(default)]
    pub enabled: bool,

    /// S3 bucket name
    pub bucket: Option<String>,

    /// S3 key prefix (e.g., "config/")
    pub prefix: Option<String>,

    /// AWS region
    pub region: Option<String>,

    /// S3 endpoint URL (for MinIO, LocalStack, or S3-compatible services)
    pub endpoint: Option<String>,

    /// Polling interval in seconds for checking configuration changes
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

fn default_poll_interval() -> u64 {
    30
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            enabled: false,
            bucket: None,
            prefix: None,
            region: None,
            endpoint: None,
            poll_interval_secs: 30,
        }
    }
}

impl S3Config {
    /// Create a new S3Config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge CLI arguments into this config (CLI takes precedence)
    pub fn merge_cli(&mut self, cli: &Cli) {
        if let Some(enabled) = cli.s3_enabled {
            self.enabled = enabled;
        }
        if cli.s3_bucket.is_some() {
            self.bucket = cli.s3_bucket.clone();
        }
        if cli.s3_prefix.is_some() {
            self.prefix = cli.s3_prefix.clone();
        }
        if cli.s3_region.is_some() {
            self.region = cli.s3_region.clone();
        }
        if cli.s3_endpoint.is_some() {
            self.endpoint = cli.s3_endpoint.clone();
        }
        if let Some(interval) = cli.s3_poll_interval {
            self.poll_interval_secs = interval;
        }
    }

    /// Validate the S3 configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.enabled {
            if self.bucket.is_none() || self.bucket.as_ref().map(|b| b.is_empty()).unwrap_or(true) {
                errors.push("S3 bucket is required when S3 is enabled".to_string());
            }

            if self.poll_interval_secs == 0 {
                errors.push("S3 poll interval must be greater than 0".to_string());
            }

            // Validate bucket name format (basic validation)
            if let Some(bucket) = &self.bucket {
                if bucket.len() < 3 || bucket.len() > 63 {
                    errors.push("S3 bucket name must be between 3 and 63 characters".to_string());
                }
                if !bucket
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
                {
                    errors.push(
                        "S3 bucket name must contain only lowercase letters, numbers, hyphens, and periods"
                            .to_string(),
                    );
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check if S3 is effectively enabled (enabled flag and bucket configured)
    pub fn is_active(&self) -> bool {
        self.enabled && self.bucket.is_some()
    }

    /// Get the full prefix path, ensuring it ends with /
    pub fn get_prefix(&self) -> String {
        match &self.prefix {
            Some(p) if !p.is_empty() => {
                if p.ends_with('/') {
                    p.clone()
                } else {
                    format!("{}/", p)
                }
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = S3Config::default();
        assert!(!config.enabled);
        assert!(config.bucket.is_none());
        assert_eq!(config.poll_interval_secs, 30);
    }

    #[test]
    fn test_validate_disabled() {
        let config = S3Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_enabled_no_bucket() {
        let config = S3Config {
            enabled: true,
            bucket: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|e| e.contains("bucket is required")));
    }

    #[test]
    fn test_validate_enabled_with_bucket() {
        let config = S3Config {
            enabled: true,
            bucket: Some("my-bucket".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_bucket_name() {
        let config = S3Config {
            enabled: true,
            bucket: Some("UPPERCASE".to_string()),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_prefix() {
        let config = S3Config {
            prefix: Some("config".to_string()),
            ..Default::default()
        };
        assert_eq!(config.get_prefix(), "config/");

        let config_with_slash = S3Config {
            prefix: Some("config/".to_string()),
            ..Default::default()
        };
        assert_eq!(config_with_slash.get_prefix(), "config/");

        let config_empty = S3Config::default();
        assert_eq!(config_empty.get_prefix(), "");
    }

    #[test]
    fn test_is_active() {
        let config = S3Config::default();
        assert!(!config.is_active());

        let config_enabled_no_bucket = S3Config {
            enabled: true,
            ..Default::default()
        };
        assert!(!config_enabled_no_bucket.is_active());

        let config_active = S3Config {
            enabled: true,
            bucket: Some("my-bucket".to_string()),
            ..Default::default()
        };
        assert!(config_active.is_active());
    }
}
