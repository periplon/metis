//! Auto-Create Session Manager
//!
//! This module provides a custom session manager that wraps `LocalSessionManager`
//! and automatically creates sessions when they are not found, instead of returning
//! an error.
//!
//! This is useful for clients that may have stale session IDs (e.g., after server restart)
//! or for simplified client implementations that don't track session lifecycle.

use futures::Stream;
use rmcp::{
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    transport::{
        common::server_side_http::ServerSseMessage,
        streamable_http_server::{
            session::{
                local::{
                    create_local_session, LocalSessionHandle, LocalSessionManagerError,
                    SessionConfig,
                },
                SessionManager,
            },
            SessionId,
        },
        WorkerTransport,
    },
};
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Error type for AutoCreateSessionManager
#[derive(Debug, Error)]
pub enum AutoCreateSessionManagerError {
    #[error("Session manager error: {0}")]
    SessionError(#[from] LocalSessionManagerError),
    #[error("Failed to create session")]
    FailedToCreateSession,
}

/// Session manager that automatically creates sessions when not found
///
/// Unlike `LocalSessionManager` which returns SessionNotFound errors,
/// this manager creates sessions on-demand when a client provides
/// a session ID that doesn't exist.
pub struct AutoCreateSessionManager {
    /// Active sessions indexed by session ID
    pub sessions: RwLock<HashMap<SessionId, LocalSessionHandle>>,
    /// Configuration for new sessions
    pub session_config: SessionConfig,
}

impl Default for AutoCreateSessionManager {
    fn default() -> Self {
        Self::new(SessionConfig::default())
    }
}

impl AutoCreateSessionManager {
    /// Create a new AutoCreateSessionManager with the given session configuration
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            session_config: config,
        }
    }

    /// Create a new AutoCreateSessionManager with default configuration
    pub fn with_defaults() -> Self {
        Self::default()
    }

    /// Ensure a session exists for the given ID, creating one if necessary
    ///
    /// This is the key auto-create functionality. If a session doesn't exist
    /// for the given ID, one is created and registered.
    async fn ensure_session_exists(
        &self,
        id: &SessionId,
    ) -> Result<(), AutoCreateSessionManagerError> {
        // Fast path: check if session already exists
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(id) {
                return Ok(());
            }
        }

        // Session doesn't exist, create one with the requested ID
        info!(session_id = %id, "Auto-creating session for client request");

        let (handle, worker) = create_local_session(id.clone(), self.session_config.clone());

        // Spawn the session worker - this is important for the session to function
        // The WorkerTransport::spawn will run the worker until closed
        let _transport = WorkerTransport::spawn(worker);

        // Register the session
        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), handle);
        debug!(session_id = %id, "Session auto-created and registered");

        Ok(())
    }
}

impl SessionManager for AutoCreateSessionManager {
    type Error = AutoCreateSessionManagerError;
    type Transport = WorkerTransport<rmcp::transport::streamable_http_server::session::local::LocalSessionWorker>;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        use rmcp::transport::common::server_side_http::session_id;

        let id = session_id();
        let (handle, worker) = create_local_session(id.clone(), self.session_config.clone());

        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), handle);
        info!(session_id = %id, "Created new session");

        Ok((id, WorkerTransport::spawn(worker)))
    }

    async fn initialize_session(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<ServerJsonRpcMessage, Self::Error> {
        self.ensure_session_exists(id).await?;

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;

        handle
            .initialize(message)
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))
    }

    async fn has_session(&self, id: &SessionId) -> Result<bool, Self::Error> {
        // Auto-create: ensure session exists, then return true
        self.ensure_session_exists(id).await?;
        Ok(true)
    }

    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        let mut sessions = self.sessions.write().await;
        if let Some(handle) = sessions.remove(id) {
            handle
                .close()
                .await
                .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))?;
            debug!(session_id = %id, "Session closed");
        }
        Ok(())
    }

    async fn create_stream(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        self.ensure_session_exists(id).await?;

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;

        let receiver = handle
            .establish_request_wise_channel()
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))?;

        handle
            .push_message(message, receiver.http_request_id)
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))?;

        Ok(tokio_stream::wrappers::ReceiverStream::new(receiver.inner))
    }

    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        self.ensure_session_exists(id).await?;

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;

        let receiver = handle
            .establish_common_channel()
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))?;

        Ok(tokio_stream::wrappers::ReceiverStream::new(receiver.inner))
    }

    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<impl Stream<Item = ServerSseMessage> + Send + 'static, Self::Error> {
        self.ensure_session_exists(id).await?;

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;

        let event_id = last_event_id
            .parse()
            .map_err(LocalSessionManagerError::InvalidEventId)?;

        let receiver = handle
            .resume(event_id)
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))?;

        Ok(tokio_stream::wrappers::ReceiverStream::new(receiver.inner))
    }

    async fn accept_message(
        &self,
        id: &SessionId,
        message: ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        self.ensure_session_exists(id).await?;

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(id)
            .ok_or(LocalSessionManagerError::SessionNotFound(id.clone()))?;

        handle
            .push_message(message, None)
            .await
            .map_err(|e| AutoCreateSessionManagerError::SessionError(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_create_session_manager_default() {
        let manager = AutoCreateSessionManager::default();

        // Check that a non-existent session "exists" (because of auto-create behavior)
        let fake_id: SessionId = "fake-session-123".to_string().into();
        let exists = manager.has_session(&fake_id).await.unwrap();
        assert!(exists, "Auto-create manager should report session exists");

        // Verify the session was actually created
        let sessions = manager.sessions.read().await;
        assert!(
            sessions.contains_key(&fake_id),
            "Session should have been auto-created"
        );
    }

    #[tokio::test]
    async fn test_create_session() {
        let manager = AutoCreateSessionManager::default();

        // Create a real session
        let (session_id, _transport) = manager.create_session().await.unwrap();

        // Verify it exists
        let sessions = manager.sessions.read().await;
        assert!(
            sessions.contains_key(&session_id),
            "Created session should exist"
        );
    }

    #[tokio::test]
    async fn test_close_session() {
        let manager = AutoCreateSessionManager::default();

        // Create a session
        let (session_id, _transport) = manager.create_session().await.unwrap();

        // Close it
        manager.close_session(&session_id).await.unwrap();

        // Verify it's gone
        let sessions = manager.sessions.read().await;
        assert!(
            !sessions.contains_key(&session_id),
            "Closed session should not exist"
        );
    }
}
