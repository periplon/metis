use crate::adapters::mcp_client::McpClientManager;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::workflow_engine::WorkflowEngine;
use crate::agents::domain::AgentPort;
use crate::config::{Settings, ToolConfig, WorkflowConfig};
use crate::domain::{Tool, ToolPort};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

/// Prefix for agent tools
pub const AGENT_TOOL_PREFIX: &str = "agent_";

/// Inner tool handler that only handles regular (non-workflow) tools.
/// This prevents circular dependency when WorkflowEngine needs to call tools.
struct InnerToolHandler {
    settings: Arc<RwLock<Settings>>,
    mock_strategy: Arc<MockStrategyHandler>,
}

impl InnerToolHandler {
    fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        Self {
            settings,
            mock_strategy,
        }
    }

    async fn find_tool_config(&self, name: &str) -> Option<ToolConfig> {
        let settings = self.settings.read().await;
        settings.tools.iter().find(|t| t.name == name).cloned()
    }
}

#[async_trait]
impl ToolPort for InnerToolHandler {
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        let settings = self.settings.read().await;
        let tools = settings
            .tools
            .iter()
            .map(|t| Tool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
            })
            .collect();
        Ok(tools)
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        if let Some(config) = self.find_tool_config(name).await {
            if let Some(mock_config) = &config.mock {
                self.mock_strategy.generate(mock_config, Some(&args)).await
            } else if let Some(static_response) = &config.static_response {
                Ok(static_response.clone())
            } else {
                Ok(Value::Null)
            }
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
}

/// Main tool handler that combines regular tools, workflow tools, MCP tools, and agents.
/// Workflows and agents are exposed as tools that can be called via MCP or by other agents.
pub struct BasicToolHandler {
    settings: Arc<RwLock<Settings>>,
    inner_handler: Arc<InnerToolHandler>,
    workflow_engine: OnceLock<Arc<WorkflowEngine>>,
    mcp_client: Arc<McpClientManager>,
    /// Optional agent handler for exposing agents as tools
    agent_handler: Arc<RwLock<Option<Arc<dyn AgentPort>>>>,
}

impl BasicToolHandler {
    pub fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        let inner_handler = Arc::new(InnerToolHandler::new(settings.clone(), mock_strategy));
        Self {
            settings,
            inner_handler,
            workflow_engine: OnceLock::new(),
            mcp_client: Arc::new(McpClientManager::new()),
            agent_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with an existing MCP client manager
    pub fn with_mcp_client(
        settings: Arc<RwLock<Settings>>,
        mock_strategy: Arc<MockStrategyHandler>,
        mcp_client: Arc<McpClientManager>,
    ) -> Self {
        let inner_handler = Arc::new(InnerToolHandler::new(settings.clone(), mock_strategy));
        Self {
            settings,
            inner_handler,
            workflow_engine: OnceLock::new(),
            mcp_client,
            agent_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the agent handler to expose agents as tools
    pub async fn set_agent_handler(&self, handler: Arc<dyn AgentPort>) {
        *self.agent_handler.write().await = Some(handler);
    }

    /// Reinitialize agents (e.g., after API keys change)
    /// This causes agents to be recreated with updated credentials
    pub async fn reinitialize_agents(&self) -> Result<()> {
        if let Some(handler) = self.agent_handler.read().await.as_ref() {
            handler.reinitialize().await?;
            tracing::info!("Agents reinitialized successfully");
        }
        Ok(())
    }

    /// Get the MCP client manager
    pub fn mcp_client(&self) -> &Arc<McpClientManager> {
        &self.mcp_client
    }

    /// Initialize MCP connections (should be called after construction)
    pub async fn initialize_mcp(&self) -> Result<()> {
        let settings = self.settings.read().await;
        self.mcp_client.initialize(&settings.mcp_servers).await
    }

    /// Get or initialize the workflow engine (lazy initialization to break circular dep)
    fn get_workflow_engine(&self) -> &Arc<WorkflowEngine> {
        self.workflow_engine.get_or_init(|| {
            Arc::new(WorkflowEngine::new(self.inner_handler.clone()))
        })
    }

    async fn find_tool_config(&self, name: &str) -> Option<ToolConfig> {
        let settings = self.settings.read().await;
        settings.tools.iter().find(|t| t.name == name).cloned()
    }

    async fn find_workflow_config(&self, name: &str) -> Option<WorkflowConfig> {
        let settings = self.settings.read().await;
        settings.workflows.iter().find(|w| w.name == name).cloned()
    }

    /// Check if a name refers to a workflow
    async fn is_workflow(&self, name: &str) -> bool {
        let settings = self.settings.read().await;
        settings.workflows.iter().any(|w| w.name == name)
    }

    /// Get MCP tools matching the given specifications
    /// Format: "server_name:tool_name" or "server_name:*" for all
    pub async fn get_mcp_tools(&self, specs: &[String]) -> Vec<Tool> {
        self.mcp_client.get_tools_for_specs(specs).await
    }
}

#[async_trait]
impl ToolPort for BasicToolHandler {
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        let settings = self.settings.read().await;

        // Regular tools
        let mut tools: Vec<Tool> = settings
            .tools
            .iter()
            .map(|t| Tool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
            })
            .collect();

        // Workflow tools (workflows exposed as tools)
        for workflow in &settings.workflows {
            tools.push(Tool {
                name: workflow.name.clone(),
                description: format!("[Workflow] {}", workflow.description),
                input_schema: workflow.input_schema.clone(),
                output_schema: workflow.output_schema.clone(),
            });
        }

        // Drop the settings lock before async call
        drop(settings);

        // Agent tools (agents exposed as tools with agent_ prefix)
        if let Some(agent_handler) = self.agent_handler.read().await.as_ref() {
            if let Ok(agents) = agent_handler.list_agents().await {
                for agent in agents {
                    // Build input schema for agent
                    // Schema must have "type": "object" to be valid
                    let has_valid_schema = agent.input_schema
                        .as_object()
                        .map(|obj| obj.contains_key("type"))
                        .unwrap_or(false);

                    let input_schema = if has_valid_schema {
                        agent.input_schema.clone()
                    } else {
                        json!({
                            "type": "object",
                            "properties": {
                                "prompt": {
                                    "type": "string",
                                    "description": "The input prompt for the agent"
                                },
                                "session_id": {
                                    "type": "string",
                                    "description": "Optional session ID for multi-turn conversations"
                                }
                            },
                            "required": ["prompt"]
                        })
                    };

                    tools.push(Tool {
                        name: format!("{}{}", AGENT_TOOL_PREFIX, agent.name),
                        description: format!("[Agent:{}] {}", agent.agent_type, agent.description),
                        input_schema,
                        output_schema: agent.output_schema,
                    });
                }
            }
        }

        // MCP tools from external servers
        let mcp_tools = self.mcp_client.list_all_tools().await;
        for (_, tool) in mcp_tools {
            tools.push(tool);
        }

        Ok(tools)
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        // Check if this is an agent tool
        if let Some(agent_name) = name.strip_prefix(AGENT_TOOL_PREFIX) {
            if let Some(agent_handler) = self.agent_handler.read().await.as_ref() {
                // Get agent info to check schemas
                let agent_info = agent_handler.get_agent(agent_name).await?;
                let agent_info = agent_info.ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_name))?;

                // Extract session_id if provided
                let session_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Transform input: if custom input_schema, serialize structured input to prompt
                let agent_input = transform_agent_input(&args, &agent_info.input_schema);

                // Execute the agent
                let response = agent_handler
                    .execute(agent_name, agent_input, session_id)
                    .await?;

                // Transform output: if output_schema defined, try to match output to schema
                let output = transform_agent_output(&response.output, &agent_info.output_schema);

                // Return agent response as tool result
                return Ok(json!({
                    "output": output,
                    "session_id": response.session_id,
                    "iterations": response.iterations,
                    "tool_calls": response.tool_calls.len(),
                    "reasoning_steps": response.reasoning_steps.len(),
                    "execution_time_ms": response.execution_time_ms
                }));
            } else {
                return Err(anyhow::anyhow!("Agent handler not available for agent: {}", agent_name));
            }
        }

        // Check if this is an MCP tool
        if McpClientManager::is_mcp_tool(name) {
            return self.mcp_client.call_tool(name, args).await;
        }

        // Check if this is a workflow
        if self.is_workflow(name).await {
            if let Some(workflow) = self.find_workflow_config(name).await {
                let engine = self.get_workflow_engine();
                return engine.execute(&workflow, args).await;
            }
        }

        // Otherwise, treat as regular tool
        if let Some(config) = self.find_tool_config(name).await {
            if let Some(mock_config) = &config.mock {
                self.inner_handler
                    .mock_strategy
                    .generate(mock_config, Some(&args))
                    .await
            } else if let Some(static_response) = &config.static_response {
                Ok(static_response.clone())
            } else {
                Ok(Value::Null)
            }
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
}

/// Transform structured input to agent prompt format
/// If input has custom schema fields (not just "prompt"), serialize them to a prompt string
fn transform_agent_input(args: &Value, _input_schema: &Value) -> Value {
    // Pass through the structured input as-is
    // The agent's prompt_template (if defined) will be used by render_user_prompt()
    // to transform structured fields into the actual prompt sent to the LLM
    args.clone()
}

/// Transform agent output to match output schema if defined
/// Returns the output content, attempting to parse as JSON if output_schema expects structured data
fn transform_agent_output(output: &Value, output_schema: &Option<Value>) -> Value {
    // Get the content from output
    let content = output
        .get("content")
        .cloned()
        .unwrap_or_else(|| output.clone());

    // If no output schema, return as-is
    let schema = match output_schema {
        Some(s) => s,
        None => return content,
    };

    // Check if schema expects an object type
    let expects_object = schema
        .get("type")
        .and_then(|t| t.as_str())
        .map(|t| t == "object")
        .unwrap_or(false);

    if !expects_object {
        return content;
    }

    // Try to parse content as JSON if it's a string
    if let Some(content_str) = content.as_str() {
        // Try to extract JSON from the content (might be wrapped in markdown code blocks)
        let json_str = extract_json_from_text(content_str);

        if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
            // Successfully parsed as JSON - return structured output
            return parsed;
        }
    }

    // If content is already an object, return it
    if content.is_object() {
        return content;
    }

    // Could not parse as structured output - return raw content
    content
}

/// Extract JSON from text that might contain markdown code blocks
fn extract_json_from_text(text: &str) -> String {
    let text = text.trim();

    // Try to find JSON in code blocks
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start + 7..].find("```") {
            return text[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try generic code block
    if let Some(start) = text.find("```") {
        let after_start = start + 3;
        // Skip language identifier if present
        let content_start = text[after_start..]
            .find('\n')
            .map(|i| after_start + i + 1)
            .unwrap_or(after_start);
        if let Some(end) = text[content_start..].find("```") {
            return text[content_start..content_start + end].trim().to_string();
        }
    }

    // Try to find raw JSON (starts with { or [)
    if text.starts_with('{') || text.starts_with('[') {
        return text.to_string();
    }

    // Return original text
    text.to_string()
}
