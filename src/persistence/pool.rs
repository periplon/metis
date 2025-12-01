//! Database connection pool management

use crate::persistence::error::PersistenceError;
use sqlx::{AnyPool, any::AnyPoolOptions};
use std::time::Duration;

/// Database backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    /// SQLite database
    Sqlite,
    /// PostgreSQL database
    Postgres,
    /// MySQL database
    Mysql,
}

impl DatabaseBackend {
    /// Detect the database backend from a connection URL
    pub fn from_url(url: &str) -> Result<Self, PersistenceError> {
        if url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else if url.starts_with("postgres:") || url.starts_with("postgresql:") {
            Ok(Self::Postgres)
        } else if url.starts_with("mysql:") || url.starts_with("mariadb:") {
            Ok(Self::Mysql)
        } else {
            Err(PersistenceError::Connection(format!(
                "Unsupported database URL format. Expected sqlite://, postgres://, or mysql://. Got: {}",
                url.split(':').next().unwrap_or("unknown")
            )))
        }
    }

    /// Get the backend name for display
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sqlite => "SQLite",
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL",
        }
    }
}

/// Connection pool wrapper with backend information
pub struct ConnectionPool {
    pool: AnyPool,
    backend: DatabaseBackend,
}

impl ConnectionPool {
    /// Create a new connection pool from a database URL
    ///
    /// # Arguments
    ///
    /// * `url` - Database connection URL (sqlite://, postgres://, mysql://)
    /// * `max_connections` - Maximum number of connections in the pool
    /// * `connect_timeout_secs` - Connection timeout in seconds
    pub async fn new(
        url: &str,
        max_connections: u32,
        connect_timeout_secs: u64,
    ) -> Result<Self, PersistenceError> {
        // Install default drivers for sqlx::any
        sqlx::any::install_default_drivers();

        // Detect backend from URL
        let backend = DatabaseBackend::from_url(url)?;

        tracing::info!(
            "Connecting to {} database with max {} connections",
            backend.name(),
            max_connections
        );

        // Build connection pool
        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(connect_timeout_secs))
            .connect(url)
            .await
            .map_err(|e| PersistenceError::Connection(e.to_string()))?;

        tracing::info!("Successfully connected to {} database", backend.name());

        Ok(Self { pool, backend })
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Get the database backend type
    pub fn backend(&self) -> DatabaseBackend {
        self.backend
    }

    /// Check if the database connection is healthy
    pub async fn health_check(&self) -> Result<(), PersistenceError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| PersistenceError::Connection(format!("Health check failed: {}", e)))?;
        Ok(())
    }

    /// Close the connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            backend: self.backend,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_detection() {
        assert_eq!(
            DatabaseBackend::from_url("sqlite://test.db").unwrap(),
            DatabaseBackend::Sqlite
        );
        assert_eq!(
            DatabaseBackend::from_url("sqlite::memory:").unwrap(),
            DatabaseBackend::Sqlite
        );
        assert_eq!(
            DatabaseBackend::from_url("postgres://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("postgresql://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("mysql://localhost/db").unwrap(),
            DatabaseBackend::Mysql
        );
        assert!(DatabaseBackend::from_url("unknown://localhost").is_err());
    }
}
