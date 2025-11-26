//! MCP Client Adapter
//!
//! This module provides client functionality to connect to external MCP servers
//! and call their tools. It manages connections to multiple MCP servers and
//! routes tool calls appropriately.

use crate::config::McpServerConfig;
use crate::domain::Tool;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Prefix for MCP tools when exposed to agents
pub const MCP_TOOL_PREFIX: &str = "mcp__";

/// MCP JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

/// Tool information from an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<Value>,
}

/// List tools response from MCP server
#[derive(Debug, Deserialize)]
struct ListToolsResult {
    tools: Vec<McpTool>,
}

/// Call tool response from MCP server
#[derive(Debug, Deserialize)]
struct CallToolResult {
    content: Vec<ContentItem>,
    #[serde(rename = "isError")]
    #[allow(dead_code)]
    is_error: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    #[allow(dead_code)]
    data: Option<String>,
    #[serde(rename = "mimeType")]
    #[allow(dead_code)]
    mime_type: Option<String>,
}

/// Connection state for an MCP server
struct McpConnection {
    config: McpServerConfig,
    client: Client,
    tools: Vec<McpTool>,
    request_id: u64,
}

impl McpConnection {
    fn new(config: McpServerConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_seconds);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();

        Self {
            config,
            client,
            tools: Vec::new(),
            request_id: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    fn get_api_key(&self) -> Option<String> {
        // First check direct api_key
        if let Some(key) = &self.config.api_key {
            return Some(key.clone());
        }

        // Then check environment variable
        if let Some(env_var) = &self.config.api_key_env {
            return std::env::var(env_var).ok();
        }

        None
    }

    async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: self.next_id(),
            method: method.to_string(),
            params,
        };

        let mut req_builder = self.client.post(&self.config.url).json(&request);

        // Add authorization header if API key is configured
        if let Some(api_key) = self.get_api_key() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "MCP server {} returned error {}: {}",
                self.config.name,
                status,
                text
            ));
        }

        let json_response: JsonRpcResponse = response.json().await?;

        if let Some(error) = json_response.error {
            return Err(anyhow::anyhow!(
                "MCP error from {}: [{}] {}",
                self.config.name,
                error.code,
                error.message
            ));
        }

        json_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in MCP response from {}", self.config.name))
    }

    async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.send_request("tools/list", None).await?;
        let list_result: ListToolsResult = serde_json::from_value(result)?;
        Ok(list_result.tools)
    }

    async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params)).await?;
        let call_result: CallToolResult = serde_json::from_value(result)?;

        // Extract text content from the response
        let mut output = String::new();
        for item in call_result.content {
            if item.content_type == "text" {
                if let Some(text) = item.text {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&text);
                }
            }
        }

        // Try to parse as JSON, otherwise return as string
        if let Ok(json_value) = serde_json::from_str::<Value>(&output) {
            Ok(json_value)
        } else {
            Ok(Value::String(output))
        }
    }
}

/// Manager for MCP client connections
pub struct McpClientManager {
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
}

impl McpClientManager {
    /// Create a new MCP client manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize connections to configured MCP servers
    pub async fn initialize(&self, configs: &[McpServerConfig]) -> Result<()> {
        let mut connections = self.connections.write().await;

        for config in configs {
            if !config.enabled {
                info!("MCP server '{}' is disabled, skipping", config.name);
                continue;
            }

            info!("Initializing MCP client for server: {}", config.name);
            let mut connection = McpConnection::new(config.clone());

            // Try to fetch tools from the server
            match connection.list_tools().await {
                Ok(tools) => {
                    info!(
                        "Connected to MCP server '{}' with {} tools",
                        config.name,
                        tools.len()
                    );
                    for tool in &tools {
                        debug!("  - {}: {:?}", tool.name, tool.description);
                    }
                    connection.tools = tools;
                    connections.insert(config.name.clone(), connection);
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to MCP server '{}': {}",
                        config.name, e
                    );
                    // Still add the connection so we can retry later
                    connections.insert(config.name.clone(), connection);
                }
            }
        }

        Ok(())
    }

    /// Refresh tools from all connected servers
    pub async fn refresh_tools(&self) -> Result<()> {
        let mut connections = self.connections.write().await;

        for (name, connection) in connections.iter_mut() {
            match connection.list_tools().await {
                Ok(tools) => {
                    info!(
                        "Refreshed tools for MCP server '{}': {} tools",
                        name,
                        tools.len()
                    );
                    connection.tools = tools;
                }
                Err(e) => {
                    error!("Failed to refresh tools for MCP server '{}': {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// List all tools from all connected MCP servers
    pub async fn list_all_tools(&self) -> Vec<(String, Tool)> {
        let connections = self.connections.read().await;
        let mut all_tools = Vec::new();

        for (server_name, connection) in connections.iter() {
            for tool in &connection.tools {
                let prefixed_name = format!("{}{}_{}", MCP_TOOL_PREFIX, server_name, tool.name);
                all_tools.push((
                    prefixed_name.clone(),
                    Tool {
                        name: prefixed_name,
                        description: tool
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("MCP tool from {}", server_name)),
                        input_schema: tool
                            .input_schema
                            .clone()
                            .unwrap_or_else(|| json!({"type": "object"})),
                        output_schema: None,
                    },
                ));
            }
        }

        all_tools
    }

    /// List tools from a specific server
    pub async fn list_server_tools(&self, server_name: &str) -> Vec<Tool> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(server_name) {
            connection
                .tools
                .iter()
                .map(|t| {
                    let prefixed_name = format!("{}{}_{}", MCP_TOOL_PREFIX, server_name, t.name);
                    Tool {
                        name: prefixed_name,
                        description: t
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("MCP tool from {}", server_name)),
                        input_schema: t
                            .input_schema
                            .clone()
                            .unwrap_or_else(|| json!({"type": "object"})),
                        output_schema: None,
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get tools that match the mcp_tools specification
    /// Format: "server_name:tool_name" or "server_name:*" for all tools from a server
    pub async fn get_tools_for_specs(&self, specs: &[String]) -> Vec<Tool> {
        let connections = self.connections.read().await;
        let mut tools = Vec::new();

        for spec in specs {
            if let Some((server_name, tool_pattern)) = spec.split_once(':') {
                if let Some(connection) = connections.get(server_name) {
                    if tool_pattern == "*" {
                        // All tools from this server
                        for t in &connection.tools {
                            let prefixed_name =
                                format!("{}{}_{}", MCP_TOOL_PREFIX, server_name, t.name);
                            tools.push(Tool {
                                name: prefixed_name,
                                description: t
                                    .description
                                    .clone()
                                    .unwrap_or_else(|| format!("MCP tool from {}", server_name)),
                                input_schema: t
                                    .input_schema
                                    .clone()
                                    .unwrap_or_else(|| json!({"type": "object"})),
                                output_schema: None,
                            });
                        }
                    } else {
                        // Specific tool
                        if let Some(t) = connection.tools.iter().find(|t| t.name == tool_pattern) {
                            let prefixed_name =
                                format!("{}{}_{}", MCP_TOOL_PREFIX, server_name, t.name);
                            tools.push(Tool {
                                name: prefixed_name,
                                description: t
                                    .description
                                    .clone()
                                    .unwrap_or_else(|| format!("MCP tool from {}", server_name)),
                                input_schema: t
                                    .input_schema
                                    .clone()
                                    .unwrap_or_else(|| json!({"type": "object"})),
                                output_schema: None,
                            });
                        }
                    }
                }
            }
        }

        tools
    }

    /// Call a tool on an MCP server
    /// Tool name format: "mcp__{server}_{tool}"
    pub async fn call_tool(&self, prefixed_name: &str, arguments: Value) -> Result<Value> {
        // Parse the prefixed name to extract server and tool
        let name_without_prefix = prefixed_name
            .strip_prefix(MCP_TOOL_PREFIX)
            .ok_or_else(|| anyhow::anyhow!("Invalid MCP tool name: {}", prefixed_name))?;

        // Find the first underscore to split server and tool name
        let (server_name, tool_name) = name_without_prefix
            .split_once('_')
            .ok_or_else(|| anyhow::anyhow!("Invalid MCP tool name format: {}", prefixed_name))?;

        let mut connections = self.connections.write().await;

        let connection = connections
            .get_mut(server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", server_name))?;

        connection.call_tool(tool_name, arguments).await
    }

    /// Check if a tool name is an MCP tool
    pub fn is_mcp_tool(name: &str) -> bool {
        name.starts_with(MCP_TOOL_PREFIX)
    }

    /// Get list of connected server names
    pub async fn list_servers(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}
