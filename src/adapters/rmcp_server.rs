//! RMCP Server Adapter
//!
//! This module provides the MCP server implementation using the official rmcp SDK.
//! It wraps the existing handler infrastructure (ResourcePort, ToolPort, PromptPort)
//! and exposes them through the standard MCP protocol.
//!
//! ## Agent Integration
//!
//! Agents are exposed as MCP tools with the prefix `agent_`. For example, an agent
//! named "assistant" becomes the tool "agent_assistant". When called, the agent
//! executes and returns its response as the tool result.

use crate::adapters::tool_handler::AGENT_TOOL_PREFIX;
use crate::agents::domain::AgentPort;
use crate::domain::{PromptPort, ResourcePort, ToolPort};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, GetPromptRequestParam, GetPromptResult,
        Implementation, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
        ListToolsResult, PaginatedRequestParam, Prompt, PromptArgument, PromptMessage,
        PromptMessageRole, RawResource, RawResourceTemplate, ReadResourceRequestParam,
        ReadResourceResult, Resource, ResourceContents, ResourceTemplate, ServerCapabilities,
        ServerInfo, Tool,
    },
    service::RequestContext,
    ErrorData as McpError, RoleServer,
};
use serde_json::json;
use tracing::info;
use std::sync::Arc;

/// Metis MCP Server
///
/// Implements the MCP ServerHandler trait using the existing handler infrastructure.
/// This provides a standards-compliant MCP server implementation.
#[derive(Clone)]
pub struct MetisServer {
    resource_handler: Arc<dyn ResourcePort>,
    tool_handler: Arc<dyn ToolPort>,
    prompt_handler: Arc<dyn PromptPort>,
    /// Optional agent handler - when present, agents are exposed as tools
    agent_handler: Option<Arc<dyn AgentPort>>,
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
            agent_handler: None,
        }
    }

    /// Create a new MetisServer with agent support
    pub fn with_agents(
        resource_handler: Arc<dyn ResourcePort>,
        tool_handler: Arc<dyn ToolPort>,
        prompt_handler: Arc<dyn PromptPort>,
        agent_handler: Arc<dyn AgentPort>,
    ) -> Self {
        Self {
            resource_handler,
            tool_handler,
            prompt_handler,
            agent_handler: Some(agent_handler),
        }
    }

    /// Set the agent handler
    pub fn set_agent_handler(&mut self, agent_handler: Arc<dyn AgentPort>) {
        self.agent_handler = Some(agent_handler);
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
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        async move {
            info!("MCP ping received");
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
        let agent_handler = self.agent_handler.clone();
        async move {
            let tools = handler
                .list_tools()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let mut mcp_tools: Vec<Tool> = tools
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

            // Add agents as tools if agent handler is present
            if let Some(agent_h) = agent_handler {
                let agents = agent_h
                    .list_agents()
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                for agent in agents {
                    let tool_name = format!("{}{}", AGENT_TOOL_PREFIX, agent.name);
                    let description = format!(
                        "[AI Agent] {} (Type: {:?}, LLM: {}/{})",
                        agent.description, agent.agent_type, agent.llm_provider, agent.llm_model
                    );

                    // Use the agent's input schema or default prompt schema
                    let schema = match agent.input_schema {
                        serde_json::Value::Object(obj) => obj,
                        _ => {
                            let mut default = serde_json::Map::new();
                            default.insert("type".to_string(), json!("object"));
                            default.insert(
                                "properties".to_string(),
                                json!({
                                    "prompt": {
                                        "type": "string",
                                        "description": "The input prompt for the agent"
                                    },
                                    "session_id": {
                                        "type": "string",
                                        "description": "Optional session ID for multi-turn conversations"
                                    }
                                }),
                            );
                            default.insert("required".to_string(), json!(["prompt"]));
                            default
                        }
                    };

                    mcp_tools.push(Tool::new(tool_name, description, schema));
                }
            }

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
        let agent_handler = self.agent_handler.clone();
        async move {
            let name = request.name.as_ref();
            let args = request
                .arguments
                .map(serde_json::Value::Object)
                .unwrap_or(serde_json::Value::Null);

            // Check if this is an agent call
            if let Some(agent_name) = name.strip_prefix(AGENT_TOOL_PREFIX) {
                if let Some(agent_h) = agent_handler {
                    // Extract session_id if provided
                    let session_id = args
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Execute the agent
                    let response = agent_h
                        .execute(agent_name, args, session_id)
                        .await
                        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

                    // Build the result with agent response details
                    let result = json!({
                        "output": response.output,
                        "session_id": response.session_id,
                        "iterations": response.iterations,
                        "execution_time_ms": response.execution_time_ms,
                        "tool_calls": response.tool_calls.len(),
                        "reasoning_steps": response.reasoning_steps.len()
                    });

                    return Ok(CallToolResult::success(vec![Content::text(result.to_string())]));
                } else {
                    return Err(McpError::invalid_params(
                        format!("Agent handler not available for agent: {}", agent_name),
                        None,
                    ));
                }
            }

            // Regular tool call
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
