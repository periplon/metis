//! File storage configuration for data lake records
//!
//! Provides configuration types for storing data lake records in local filesystem
//! or S3-compatible object storage, supporting both Parquet and JSONL formats.

use serde::{Deserialize, Serialize};

/// Storage mode for data lake records
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataLakeStorageMode {
    /// Store only in database (default, backward compatible)
    #[default]
    Database,
    /// Store only in files (Parquet/JSONL)
    File,
    /// Store in both database and files (write-through)
    Both,
}

impl DataLakeStorageMode {
    /// Check if records should be stored in the database
    pub fn uses_database(&self) -> bool {
        matches!(self, Self::Database | Self::Both)
    }

    /// Check if records should be stored in files
    pub fn uses_files(&self) -> bool {
        matches!(self, Self::File | Self::Both)
    }
}

/// File format for data lake storage
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataLakeFileFormat {
    /// Apache Parquet - columnar format, best for analytics queries
    #[default]
    Parquet,
    /// JSON Lines - one JSON record per line, human-readable
    Jsonl,
}

impl DataLakeFileFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Parquet => "parquet",
            Self::Jsonl => "jsonl",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Parquet => "application/vnd.apache.parquet",
            Self::Jsonl => "application/x-ndjson",
        }
    }
}

/// Global file storage configuration for data lakes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileStorageConfig {
    /// Whether file storage is enabled globally
    #[serde(default)]
    pub enabled: bool,

    /// Local filesystem base path (e.g., "./data")
    /// Used when S3 is not configured or for local development
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,

    /// S3 configuration for data storage
    /// When configured, takes precedence over local_path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3DataConfig>,

    /// Default file format for new data lakes
    #[serde(default)]
    pub default_format: DataLakeFileFormat,

    /// Batch size for writing records to files
    /// Controls how many records are written per file
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum file size in bytes before creating a new file
    /// Default: 128MB
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: usize,
}

fn default_batch_size() -> usize {
    1000
}

fn default_max_file_size() -> usize {
    128 * 1024 * 1024 // 128MB
}

impl FileStorageConfig {
    /// Check if the configuration is valid
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.local_path.is_none() && self.s3.is_none() {
                return Err(
                    "File storage is enabled but neither local_path nor s3 is configured"
                        .to_string(),
                );
            }
            if let Some(s3) = &self.s3 {
                s3.validate()?;
            }
        }
        Ok(())
    }

    /// Check if S3 storage should be used
    pub fn uses_s3(&self) -> bool {
        self.enabled && self.s3.is_some()
    }

    /// Check if local storage should be used
    pub fn uses_local(&self) -> bool {
        self.enabled && self.s3.is_none() && self.local_path.is_some()
    }

    /// Get the effective storage backend description
    pub fn backend_description(&self) -> &'static str {
        if !self.enabled {
            "disabled"
        } else if self.s3.is_some() {
            "S3"
        } else if self.local_path.is_some() {
            "local filesystem"
        } else {
            "unconfigured"
        }
    }
}

/// S3-specific configuration for data file storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3DataConfig {
    /// S3 bucket for data storage
    pub bucket: String,

    /// Prefix for data files (e.g., "data/")
    /// Files will be stored at: s3://{bucket}/{prefix}/data-lakes/{lake_name}/{schema_name}/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,

    /// AWS region (e.g., "us-east-1")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// S3 endpoint URL for S3-compatible services (MinIO, LocalStack, etc.)
    /// Leave empty for standard AWS S3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// Access key ID (optional, uses AWS credential chain if not provided)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,

    /// Secret access key (optional, uses AWS credential chain if not provided)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,

    /// Whether to use path-style addressing (required for some S3-compatible services)
    #[serde(default)]
    pub force_path_style: bool,

    /// Whether to allow HTTP (non-HTTPS) connections
    /// Only enable for local development with MinIO/LocalStack
    #[serde(default)]
    pub allow_http: bool,
}

impl S3DataConfig {
    /// Check if the configuration is valid
    pub fn validate(&self) -> Result<(), String> {
        if self.bucket.is_empty() {
            return Err("S3 bucket name is required".to_string());
        }
        // Check for consistent credential configuration
        match (&self.access_key_id, &self.secret_access_key) {
            (Some(_), None) => {
                return Err("access_key_id provided without secret_access_key".to_string())
            }
            (None, Some(_)) => {
                return Err("secret_access_key provided without access_key_id".to_string())
            }
            _ => {}
        }
        Ok(())
    }

    /// Get the effective prefix with trailing slash
    pub fn effective_prefix(&self) -> String {
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
    fn test_storage_mode_uses_database() {
        assert!(DataLakeStorageMode::Database.uses_database());
        assert!(!DataLakeStorageMode::File.uses_database());
        assert!(DataLakeStorageMode::Both.uses_database());
    }

    #[test]
    fn test_storage_mode_uses_files() {
        assert!(!DataLakeStorageMode::Database.uses_files());
        assert!(DataLakeStorageMode::File.uses_files());
        assert!(DataLakeStorageMode::Both.uses_files());
    }

    #[test]
    fn test_file_format_extension() {
        assert_eq!(DataLakeFileFormat::Parquet.extension(), "parquet");
        assert_eq!(DataLakeFileFormat::Jsonl.extension(), "jsonl");
    }

    #[test]
    fn test_file_storage_config_validation() {
        let config = FileStorageConfig {
            enabled: true,
            local_path: None,
            s3: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = FileStorageConfig {
            enabled: true,
            local_path: Some("./data".to_string()),
            s3: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_s3_config_validation() {
        let config = S3DataConfig {
            bucket: "".to_string(),
            prefix: None,
            region: None,
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert!(config.validate().is_err());

        let config = S3DataConfig {
            bucket: "my-bucket".to_string(),
            prefix: None,
            region: None,
            endpoint: None,
            access_key_id: Some("key".to_string()),
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert!(config.validate().is_err());

        let config = S3DataConfig {
            bucket: "my-bucket".to_string(),
            prefix: Some("data".to_string()),
            region: Some("us-east-1".to_string()),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_s3_effective_prefix() {
        let config = S3DataConfig {
            bucket: "bucket".to_string(),
            prefix: None,
            region: None,
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert_eq!(config.effective_prefix(), "");

        let config = S3DataConfig {
            bucket: "bucket".to_string(),
            prefix: Some("data".to_string()),
            region: None,
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert_eq!(config.effective_prefix(), "data/");

        let config = S3DataConfig {
            bucket: "bucket".to_string(),
            prefix: Some("data/".to_string()),
            region: None,
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            force_path_style: false,
            allow_http: false,
        };
        assert_eq!(config.effective_prefix(), "data/");
    }
}
