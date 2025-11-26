//! Agent handler implementing AgentPort

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::agents::config::{AgentConfig, OrchestrationConfig};
use crate::agents::core::{create_agent, Agent};
use crate::agents::domain::{
    AgentInfo, AgentPort, AgentResponse, AgentStream, ConversationSession, SessionSummary,
};
use crate::agents::error::{AgentError, AgentResult};
use crate::agents::llm::{create_provider, LlmProvider};
use crate::agents::memory::{create_store, ConversationStore};
use crate::agents::orchestration::OrchestrationEngine;
use crate::config::Settings;
use crate::domain::ToolPort;

/// Handler for agent operations
pub struct AgentHandler {
    settings: Arc<RwLock<Settings>>,
    tool_handler: Arc<dyn ToolPort>,
    /// Cached agents (name -> agent)
    agents: Arc<RwLock<HashMap<String, Arc<dyn Agent>>>>,
    /// Cached LLM providers (config hash -> provider)
    providers: Arc<RwLock<HashMap<String, Arc<dyn LlmProvider>>>>,
    /// Default memory store
    default_store: Arc<dyn ConversationStore>,
    /// Orchestration engine
    orchestration: Arc<RwLock<Option<OrchestrationEngine>>>,
}

impl AgentHandler {
    /// Create a new agent handler
    pub fn new(settings: Arc<RwLock<Settings>>, tool_handler: Arc<dyn ToolPort>) -> Self {
        let default_store = Arc::new(crate::agents::memory::InMemoryStore::new(100));

        Self {
            settings,
            tool_handler,
            agents: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_store,
            orchestration: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize agents from configuration
    pub async fn initialize(&self) -> AgentResult<()> {
        let settings = self.settings.read().await;

        let mut agents = HashMap::new();

        for config in &settings.agents {
            match self.create_agent_from_config(config).await {
                Ok(agent) => {
                    agents.insert(config.name.clone(), agent);
                }
                Err(e) => {
                    tracing::warn!("Failed to create agent '{}': {}", config.name, e);
                }
            }
        }

        // Update cached agents
        *self.agents.write().await = agents.clone();

        // Create orchestration engine
        let orchestration = OrchestrationEngine::new(agents);
        *self.orchestration.write().await = Some(orchestration);

        Ok(())
    }

    /// Create an agent from configuration
    async fn create_agent_from_config(&self, config: &AgentConfig) -> AgentResult<Arc<dyn Agent>> {
        // Get or create LLM provider
        let provider = self.get_or_create_provider(&config.llm).await?;

        // Get or create memory store
        let store = self.get_or_create_store(&config.memory).await?;

        // Create the agent
        create_agent(
            config.clone(),
            provider,
            store,
            self.tool_handler.clone(),
        )
    }

    /// Get or create an LLM provider
    async fn get_or_create_provider(
        &self,
        config: &crate::agents::config::LlmProviderConfig,
    ) -> AgentResult<Arc<dyn LlmProvider>> {
        let key = format!("{:?}_{}", config.provider, config.model);

        // Check cache
        if let Some(provider) = self.providers.read().await.get(&key) {
            return Ok(provider.clone());
        }

        // Create new provider
        let provider = create_provider(config).map_err(|e| AgentError::Configuration(e.to_string()))?;

        // Cache it
        self.providers.write().await.insert(key, provider.clone());

        Ok(provider)
    }

    /// Get or create a memory store
    async fn get_or_create_store(
        &self,
        config: &crate::agents::config::MemoryConfig,
    ) -> AgentResult<Arc<dyn ConversationStore>> {
        // For now, use default store or create based on config
        match config.backend {
            crate::agents::config::MemoryBackend::InMemory => {
                Ok(self.default_store.clone())
            }
            _ => {
                create_store(config)
            }
        }
    }

    /// Get orchestration configurations from settings
    pub async fn get_orchestrations(&self) -> Vec<OrchestrationConfig> {
        self.settings.read().await.orchestrations.clone()
    }

    /// Execute an orchestration
    pub fn execute_orchestration(&self, config: &OrchestrationConfig, input: Value) -> AgentResult<AgentStream> {
        // Use try_read() to avoid blocking in async contexts
        match self.orchestration.try_read() {
            Ok(orchestration) => {
                match orchestration.as_ref() {
                    Some(engine) => Ok(engine.execute(config, input)),
                    None => Err(AgentError::Internal("Orchestration engine not initialized".to_string())),
                }
            }
            Err(_) => Err(AgentError::Internal("Failed to acquire orchestration lock".to_string())),
        }
    }
}

#[async_trait]
impl AgentPort for AgentHandler {
    async fn execute(
        &self,
        name: &str,
        input: Value,
        session_id: Option<String>,
    ) -> anyhow::Result<AgentResponse> {
        // Use async read() to get the agent without blocking
        let agents = self.agents.read().await;

        match agents.get(name) {
            Some(agent) => {
                let agent = agent.clone();
                drop(agents); // Release the lock before executing
                let stream = agent.execute(input, session_id);
                stream.collect().await.map_err(|e| anyhow::anyhow!("{}", e))
            }
            None => Err(anyhow::anyhow!("Agent not found: {}", name)),
        }
    }

    fn execute_stream(
        &self,
        name: &str,
        input: Value,
        session_id: Option<String>,
    ) -> AgentStream {
        // Use try_read() to avoid blocking in async contexts
        // This is for streaming use cases where we can't use async
        match self.agents.try_read() {
            Ok(agents) => {
                match agents.get(name) {
                    Some(agent) => agent.execute(input, session_id),
                    None => {
                        let (sender, stream) = AgentStream::channel(1);
                        let name = name.to_string();
                        tokio::spawn(async move {
                            let _ = sender.send_error(AgentError::NotFound(name)).await;
                        });
                        stream
                    }
                }
            }
            Err(_) => {
                // Lock is held, return error stream
                let (sender, stream) = AgentStream::channel(1);
                tokio::spawn(async move {
                    let _ = sender.send_error(AgentError::Internal(
                        "Failed to acquire agent lock".to_string()
                    )).await;
                });
                stream
            }
        }
    }

    async fn list_agents(&self) -> anyhow::Result<Vec<AgentInfo>> {
        let agents = self.agents.read().await;

        Ok(agents
            .iter()
            .map(|(name, agent)| {
                let config = agent.config();
                AgentInfo {
                    name: name.clone(),
                    description: config.description.clone(),
                    agent_type: config.agent_type,
                    input_schema: config.input_schema.clone(),
                    output_schema: config.output_schema.clone(),
                    available_tools: config.available_tools.clone(),
                    mcp_tools: config.mcp_tools.clone(),
                    llm_provider: config.llm.provider.to_string(),
                    llm_model: config.llm.model.clone(),
                }
            })
            .collect())
    }

    async fn get_agent(&self, name: &str) -> anyhow::Result<Option<AgentInfo>> {
        let agents = self.agents.read().await;

        Ok(agents.get(name).map(|agent| {
            let config = agent.config();
            AgentInfo {
                name: name.to_string(),
                description: config.description.clone(),
                agent_type: config.agent_type,
                input_schema: config.input_schema.clone(),
                output_schema: config.output_schema.clone(),
                available_tools: config.available_tools.clone(),
                mcp_tools: config.mcp_tools.clone(),
                llm_provider: config.llm.provider.to_string(),
                llm_model: config.llm.model.clone(),
            }
        }))
    }

    async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<ConversationSession>> {
        self.default_store.load(session_id).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn list_sessions(
        &self,
        agent_name: &str,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<Vec<SessionSummary>> {
        self.default_store.list(Some(agent_name), limit, offset).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn delete_session(&self, session_id: &str) -> anyhow::Result<()> {
        self.default_store.delete(session_id).await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}
