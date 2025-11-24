use clap::Parser;
use std::path::PathBuf;

/// MCP Mock Server - A flexible mock server for Model Context Protocol
#[derive(Parser, Debug, Clone)]
#[command(name = "metis", version, about, long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, env = "METIS_CONFIG", default_value = "metis.toml")]
    pub config: PathBuf,

    /// Server host address
    #[arg(long, env = "METIS_HOST")]
    pub host: Option<String>,

    /// Server port
    #[arg(long, env = "METIS_PORT")]
    pub port: Option<u16>,

    /// Enable S3 configuration source
    #[arg(long, env = "METIS_S3_ENABLED", num_args = 0..=1, default_missing_value = "true")]
    pub s3_enabled: Option<bool>,

    /// S3 bucket name for configuration files
    #[arg(long, env = "METIS_S3_BUCKET")]
    pub s3_bucket: Option<String>,

    /// S3 key prefix for configuration files (e.g., "config/")
    #[arg(long, env = "METIS_S3_PREFIX")]
    pub s3_prefix: Option<String>,

    /// AWS region for S3
    #[arg(long, env = "METIS_S3_REGION")]
    pub s3_region: Option<String>,

    /// S3 endpoint URL (for MinIO, LocalStack, or other S3-compatible services)
    #[arg(long, env = "METIS_S3_ENDPOINT")]
    pub s3_endpoint: Option<String>,

    /// S3 configuration polling interval in seconds
    #[arg(long, env = "METIS_S3_POLL_INTERVAL")]
    pub s3_poll_interval: Option<u64>,
}

impl Cli {
    /// Check if any S3 configuration is provided via CLI or environment
    pub fn has_s3_config(&self) -> bool {
        self.s3_enabled.is_some()
            || self.s3_bucket.is_some()
            || self.s3_prefix.is_some()
            || self.s3_region.is_some()
            || self.s3_endpoint.is_some()
            || self.s3_poll_interval.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_defaults() {
        let cli = Cli::parse_from(["metis"]);
        assert_eq!(cli.config, PathBuf::from("metis.toml"));
        assert!(cli.host.is_none());
        assert!(cli.port.is_none());
        assert!(cli.s3_enabled.is_none());
        assert!(cli.s3_bucket.is_none());
    }

    #[test]
    fn test_cli_with_args() {
        let cli = Cli::parse_from([
            "metis",
            "--config",
            "custom.toml",
            "--host",
            "0.0.0.0",
            "--port",
            "8080",
            "--s3-enabled",
            "--s3-bucket",
            "my-bucket",
            "--s3-prefix",
            "config/",
            "--s3-region",
            "us-east-1",
            "--s3-poll-interval",
            "60",
        ]);
        assert_eq!(cli.config, PathBuf::from("custom.toml"));
        assert_eq!(cli.host, Some("0.0.0.0".to_string()));
        assert_eq!(cli.port, Some(8080));
        assert_eq!(cli.s3_enabled, Some(true));
        assert_eq!(cli.s3_bucket, Some("my-bucket".to_string()));
        assert_eq!(cli.s3_prefix, Some("config/".to_string()));
        assert_eq!(cli.s3_region, Some("us-east-1".to_string()));
        assert_eq!(cli.s3_poll_interval, Some(60));
    }

    #[test]
    fn test_has_s3_config() {
        let cli = Cli::parse_from(["metis"]);
        assert!(!cli.has_s3_config());

        let cli_with_bucket = Cli::parse_from(["metis", "--s3-bucket", "test"]);
        assert!(cli_with_bucket.has_s3_config());
    }
}
