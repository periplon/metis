//! RMCP Server Adapter
//!
//! This module provides the MCP server implementation using the official rmcp SDK.
//! It wraps the existing handler infrastructure (ResourcePort, ToolPort, PromptPort)
//! and exposes them through the standard MCP protocol.
//!
//! ## Agent Integration
//!
//! Agents are exposed as MCP tools through the ToolPort. The tool handler includes
//! agents with the prefix `agent_`. For example, an agent named "assistant" becomes
//! the tool "agent_assistant". When called, the agent executes and returns its
//! response as the tool result.
//!
//! ## List Change Notifications
//!
//! The server supports MCP list change notifications. When tools, resources, or prompts
//! change, connected clients can be notified via:
//! - `notifications/tools/list_changed`
//! - `notifications/resources/list_changed`
//! - `notifications/prompts/list_changed`
//!
//! Use `NotificationBroadcaster` to send these notifications to all connected peers.

use crate::domain::{PromptPort, ResourcePort, ToolPort};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, GetPromptRequestParam, GetPromptResult,
        Implementation, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
        ListToolsResult, PaginatedRequestParam, Prompt, PromptArgument, PromptMessage,
        PromptMessageRole, PromptListChangedNotification, RawResource, RawResourceTemplate,
        ReadResourceRequestParam, ReadResourceResult, Resource, ResourceContents,
        ResourceListChangedNotification, ResourceTemplate, ServerCapabilities, ServerInfo,
        ServerNotification, Tool, ToolListChangedNotification,
    },
    service::{Peer, RequestContext},
    ErrorData as McpError, RoleServer,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Type alias for a unique peer identifier
pub type PeerId = String;

/// Shared notification broadcaster
pub type SharedNotificationBroadcaster = Arc<NotificationBroadcaster>;

/// Notification broadcaster for MCP list change notifications
///
/// This struct maintains a list of connected peers and can broadcast
/// notifications when tools, resources, or prompts change.
///
/// # Example
///
/// ```ignore
/// let broadcaster = NotificationBroadcaster::new();
/// // ... after a tool is created via API ...
/// broadcaster.notify_tools_changed().await;
/// ```
#[derive(Default)]
pub struct NotificationBroadcaster {
    /// Connected peers indexed by their unique ID
    peers: RwLock<HashMap<PeerId, Peer<RoleServer>>>,
    /// Counter for generating unique peer IDs
    peer_counter: RwLock<u64>,
}

impl NotificationBroadcaster {
    /// Create a new notification broadcaster
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            peer_counter: RwLock::new(0),
        }
    }

    /// Register a peer and return its ID
    pub async fn register_peer(&self, peer: Peer<RoleServer>) -> PeerId {
        let mut counter = self.peer_counter.write().await;
        *counter += 1;
        let id = format!("peer_{}", *counter);

        let mut peers = self.peers.write().await;
        peers.insert(id.clone(), peer);
        debug!("Registered MCP peer: {} (total: {})", id, peers.len());
        id
    }

    /// Unregister a peer by ID
    pub async fn unregister_peer(&self, id: &str) {
        let mut peers = self.peers.write().await;
        if peers.remove(id).is_some() {
            debug!("Unregistered MCP peer: {} (remaining: {})", id, peers.len());
        }
    }

    /// Get the number of connected peers
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Notify all peers that the tool list has changed
    pub async fn notify_tools_changed(&self) {
        let notification = ServerNotification::ToolListChangedNotification(
            ToolListChangedNotification::default(),
        );
        self.broadcast(notification, "tools").await;
    }

    /// Notify all peers that the resource list has changed
    pub async fn notify_resources_changed(&self) {
        let notification = ServerNotification::ResourceListChangedNotification(
            ResourceListChangedNotification::default(),
        );
        self.broadcast(notification, "resources").await;
    }

    /// Notify all peers that the prompt list has changed
    pub async fn notify_prompts_changed(&self) {
        let notification = ServerNotification::PromptListChangedNotification(
            PromptListChangedNotification::default(),
        );
        self.broadcast(notification, "prompts").await;
    }

    /// Broadcast a notification to all connected peers
    async fn broadcast(&self, notification: ServerNotification, list_type: &str) {
        let peers = self.peers.read().await;
        if peers.is_empty() {
            debug!("No peers connected to notify about {} list change", list_type);
            return;
        }

        info!("Broadcasting {} list changed notification to {} peer(s)", list_type, peers.len());

        let mut failed_peers = Vec::new();

        for (id, peer) in peers.iter() {
            if let Err(e) = peer.send_notification(notification.clone().into()).await {
                warn!("Failed to send notification to peer {}: {}", id, e);
                failed_peers.push(id.clone());
            }
        }

        // Clean up failed peers
        drop(peers);
        if !failed_peers.is_empty() {
            let mut peers = self.peers.write().await;
            for id in failed_peers {
                peers.remove(&id);
                debug!("Removed disconnected peer: {}", id);
            }
        }
    }
}

/// Metis MCP Server
///
/// Implements the MCP ServerHandler trait using the existing handler infrastructure.
/// This provides a standards-compliant MCP server implementation.
///
/// The tool_handler includes support for:
/// - Regular tools
/// - Workflow tools
/// - Agent tools (with `agent_` prefix)
/// - MCP tools from external servers
#[derive(Clone)]
pub struct MetisServer {
    resource_handler: Arc<dyn ResourcePort>,
    tool_handler: Arc<dyn ToolPort>,
    prompt_handler: Arc<dyn PromptPort>,
    broadcaster: SharedNotificationBroadcaster,
}

impl MetisServer {
    /// Create a new MetisServer with the given handlers
    pub fn new(
        resource_handler: Arc<dyn ResourcePort>,
        tool_handler: Arc<dyn ToolPort>,
        prompt_handler: Arc<dyn PromptPort>,
    ) -> Self {
        Self {
            resource_handler,
            tool_handler,
            prompt_handler,
            broadcaster: Arc::new(NotificationBroadcaster::new()),
        }
    }

    /// Create a new MetisServer with a shared notification broadcaster
    pub fn with_broadcaster(
        resource_handler: Arc<dyn ResourcePort>,
        tool_handler: Arc<dyn ToolPort>,
        prompt_handler: Arc<dyn PromptPort>,
        broadcaster: SharedNotificationBroadcaster,
    ) -> Self {
        Self {
            resource_handler,
            tool_handler,
            prompt_handler,
            broadcaster,
        }
    }

    /// Get the notification broadcaster
    pub fn broadcaster(&self) -> &SharedNotificationBroadcaster {
        &self.broadcaster
    }
}

impl ServerHandler for MetisServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation {
                name: "metis-mock-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "Metis MCP Mock Server - A configurable mock server for MCP protocol testing"
                    .to_string(),
            ),
        }
    }

    fn ping(
        &self,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        let broadcaster = self.broadcaster.clone();
        async move {
            info!("MCP ping received");
            // Register the peer for future notifications
            let peer = context.peer.clone();
            broadcaster.register_peer(peer).await;
            Ok(())
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        let handler = self.resource_handler.clone();
        async move {
            let resources = handler
                .list_resources()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mcp_resources: Vec<Resource> = resources
                .into_iter()
                .map(|r| {
                    Resource::new(
                        RawResource {
                            uri: r.uri.into(),
                            name: r.name.into(),
                            title: None,
                            description: r.description.map(Into::into),
                            mime_type: r.mime_type.map(Into::into),
                            size: None,
                            icons: None,
                        },
                        None,
                    )
                })
                .collect();

            Ok(ListResourcesResult {
                resources: mcp_resources,
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        let handler = self.resource_handler.clone();
        async move {
            let uri = request.uri.as_str();
            let result = handler
                .get_resource(uri)
                .await
                .map_err(|e| McpError::resource_not_found(e.to_string(), None))?;

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(result.content, result.uri)],
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_
    {
        let handler = self.resource_handler.clone();
        async move {
            let templates = handler
                .list_resource_templates()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mcp_templates: Vec<ResourceTemplate> = templates
                .into_iter()
                .map(|t| {
                    ResourceTemplate::new(
                        RawResourceTemplate {
                            uri_template: t.uri_template.into(),
                            name: t.name.into(),
                            title: None,
                            description: t.description.map(Into::into),
                            mime_type: t.mime_type.map(Into::into),
                        },
                        None,
                    )
                })
                .collect();

            Ok(ListResourceTemplatesResult {
                resource_templates: mcp_templates,
                next_cursor: None,
            })
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let handler = self.tool_handler.clone();
        async move {
            // tool_handler.list_tools() already includes:
            // - Regular tools
            // - Workflow tools
            // - Agent tools (with agent_ prefix)
            // - MCP tools from external servers
            let tools = handler
                .list_tools()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mcp_tools: Vec<Tool> = tools
                .into_iter()
                .map(|t| {
                    // Input schema should be a JSON object
                    let schema = match t.input_schema {
                        serde_json::Value::Object(obj) => obj,
                        _ => serde_json::Map::new(),
                    };
                    Tool::new(t.name, t.description, schema)
                })
                .collect();

            Ok(ListToolsResult {
                tools: mcp_tools,
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let handler = self.tool_handler.clone();
        async move {
            let name = request.name.as_ref();
            let args = request
                .arguments
                .map(serde_json::Value::Object)
                .unwrap_or(serde_json::Value::Null);

            // tool_handler.execute_tool() handles:
            // - Agent tools (with agent_ prefix)
            // - MCP tools from external servers
            // - Workflow tools
            // - Regular tools
            let result = handler
                .execute_tool(name, args)
                .await
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

            let text = if let Some(s) = result.as_str() {
                s.to_string()
            } else {
                result.to_string()
            };

            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        let handler = self.prompt_handler.clone();
        async move {
            let prompts = handler
                .list_prompts()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mcp_prompts: Vec<Prompt> = prompts
                .into_iter()
                .map(|p| {
                    let args: Option<Vec<PromptArgument>> = p.arguments.map(|args| {
                        args.into_iter()
                            .map(|a| PromptArgument {
                                name: a.name.into(),
                                title: None,
                                description: a.description.map(Into::into),
                                required: Some(a.required),
                            })
                            .collect()
                    });
                    Prompt::new(p.name, Some(p.description), args)
                })
                .collect();

            Ok(ListPromptsResult {
                prompts: mcp_prompts,
                next_cursor: None,
            })
        }
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        let handler = self.prompt_handler.clone();
        async move {
            let name = request.name.as_ref();
            let args = request.arguments.map(serde_json::Value::Object);

            let result = handler
                .get_prompt(name, args)
                .await
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

            let messages: Vec<PromptMessage> = result
                .messages
                .into_iter()
                .map(|m| {
                    let role = match m.role.as_str() {
                        "assistant" => PromptMessageRole::Assistant,
                        _ => PromptMessageRole::User,
                    };
                    PromptMessage::new_text(role, m.content.text)
                })
                .collect();

            Ok(GetPromptResult {
                description: result.description.map(Into::into),
                messages,
            })
        }
    }
}
