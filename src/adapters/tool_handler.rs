use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::workflow_engine::WorkflowEngine;
use crate::config::{Settings, ToolConfig, WorkflowConfig};
use crate::domain::{Tool, ToolPort};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

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

/// Main tool handler that combines regular tools and workflow tools.
/// Workflows are exposed as tools that can be called via MCP.
pub struct BasicToolHandler {
    settings: Arc<RwLock<Settings>>,
    inner_handler: Arc<InnerToolHandler>,
    workflow_engine: OnceLock<Arc<WorkflowEngine>>,
}

impl BasicToolHandler {
    pub fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        let inner_handler = Arc::new(InnerToolHandler::new(settings.clone(), mock_strategy));
        Self {
            settings,
            inner_handler,
            workflow_engine: OnceLock::new(),
        }
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

        Ok(tools)
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        // Check if this is a workflow first
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
