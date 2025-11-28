//! API client for the Metis backend

#![allow(dead_code)]

use crate::types::*;
use gloo_net::http::Request;

const API_BASE: &str = "/api";

/// Fetch configuration overview
pub async fn get_config() -> Result<ConfigOverview, String> {
    let url = format!("{}/config", API_BASE);
    fetch_json::<ConfigOverview>(&url).await
}

/// Fetch editable server settings
pub async fn get_server_settings() -> Result<ServerSettings, String> {
    let url = format!("{}/config/settings", API_BASE);
    fetch_json::<ServerSettings>(&url).await
}

/// Update server settings
pub async fn update_server_settings(settings: &ServerSettings) -> Result<ServerSettings, String> {
    let url = format!("{}/config/settings", API_BASE);
    put_json::<ServerSettings, ServerSettings>(&url, settings).await
}

/// Save config to disk (metis.toml) with optimistic locking
/// Returns the new version number on success, or an error message on failure
pub async fn save_config_to_disk(expected_version: Option<u64>) -> Result<SaveConfigResponse, String> {
    let url = format!("{}/config/save-disk", API_BASE);
    let request = SaveConfigRequest { expected_version };
    post_json::<SaveConfigRequest, SaveConfigResponse>(&url, &request).await
}

/// Save config to S3 with optimistic locking
/// Returns the new version number on success, or an error message on failure
pub async fn save_config_to_s3(expected_version: Option<u64>) -> Result<SaveConfigResponse, String> {
    let url = format!("{}/config/save-s3", API_BASE);
    let request = SaveConfigRequest { expected_version };
    post_json::<SaveConfigRequest, SaveConfigResponse>(&url, &request).await
}

/// Export config as JSON
pub async fn export_config() -> Result<serde_json::Value, String> {
    let url = format!("{}/config/export", API_BASE);
    fetch_json::<serde_json::Value>(&url).await
}

/// Import config from JSON
pub async fn import_config(config: &serde_json::Value) -> Result<(), String> {
    let url = format!("{}/config/import", API_BASE);
    post_empty(&url, config).await
}

/// Merge result showing what was added
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MergeResult {
    pub resources_added: usize,
    pub resource_templates_added: usize,
    pub tools_added: usize,
    pub prompts_added: usize,
    pub workflows_added: usize,
    pub agents_added: usize,
    pub orchestrations_added: usize,
    pub mcp_servers_added: usize,
}

impl MergeResult {
    pub fn total_added(&self) -> usize {
        self.resources_added
            + self.resource_templates_added
            + self.tools_added
            + self.prompts_added
            + self.workflows_added
            + self.agents_added
            + self.orchestrations_added
            + self.mcp_servers_added
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.resources_added > 0 {
            parts.push(format!("{} resources", self.resources_added));
        }
        if self.resource_templates_added > 0 {
            parts.push(format!("{} templates", self.resource_templates_added));
        }
        if self.tools_added > 0 {
            parts.push(format!("{} tools", self.tools_added));
        }
        if self.prompts_added > 0 {
            parts.push(format!("{} prompts", self.prompts_added));
        }
        if self.workflows_added > 0 {
            parts.push(format!("{} workflows", self.workflows_added));
        }
        if self.agents_added > 0 {
            parts.push(format!("{} agents", self.agents_added));
        }
        if self.orchestrations_added > 0 {
            parts.push(format!("{} orchestrations", self.orchestrations_added));
        }
        if self.mcp_servers_added > 0 {
            parts.push(format!("{} MCP servers", self.mcp_servers_added));
        }
        if parts.is_empty() {
            "No new items added (all items already exist)".to_string()
        } else {
            format!("Added: {}", parts.join(", "))
        }
    }
}

/// Merge config from JSON (only adds new elements)
pub async fn merge_config(config: &serde_json::Value) -> Result<MergeResult, String> {
    let url = format!("{}/config/merge", API_BASE);
    post_json::<serde_json::Value, MergeResult>(&url, config).await
}

/// Fetch metrics as JSON
pub async fn get_metrics() -> Result<serde_json::Value, String> {
    let url = format!("{}/metrics/json", API_BASE);
    fetch_json::<serde_json::Value>(&url).await
}

// ============================================================================
// Resources
// ============================================================================

pub async fn list_resources() -> Result<Vec<Resource>, String> {
    let url = format!("{}/resources", API_BASE);
    fetch_json::<Vec<Resource>>(&url).await
}

pub async fn get_resource(uri: &str) -> Result<Resource, String> {
    let encoded_uri = urlencoding_encode(uri);
    let url = format!("{}/resources/{}", API_BASE, encoded_uri);
    fetch_json::<Resource>(&url).await
}

pub async fn create_resource(resource: &Resource) -> Result<Resource, String> {
    let url = format!("{}/resources", API_BASE);
    post_json::<Resource, Resource>(&url, resource).await
}

pub async fn update_resource(uri: &str, resource: &Resource) -> Result<Resource, String> {
    let encoded_uri = urlencoding_encode(uri);
    let url = format!("{}/resources/{}", API_BASE, encoded_uri);
    put_json::<Resource, Resource>(&url, resource).await
}

pub async fn delete_resource(uri: &str) -> Result<(), String> {
    let encoded_uri = urlencoding_encode(uri);
    let url = format!("{}/resources/{}", API_BASE, encoded_uri);
    delete_request(&url).await
}

pub async fn test_resource(uri: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let encoded_uri = urlencoding_encode(uri);
    let url = format!("{}/resources/{}/test", API_BASE, encoded_uri);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Resource Templates
// ============================================================================

pub async fn list_resource_templates() -> Result<Vec<ResourceTemplate>, String> {
    let url = format!("{}/resource-templates", API_BASE);
    fetch_json::<Vec<ResourceTemplate>>(&url).await
}

pub async fn get_resource_template(uri_template: &str) -> Result<ResourceTemplate, String> {
    let encoded_uri = urlencoding_encode(uri_template);
    let url = format!("{}/resource-templates/{}", API_BASE, encoded_uri);
    fetch_json::<ResourceTemplate>(&url).await
}

pub async fn create_resource_template(template: &ResourceTemplate) -> Result<ResourceTemplate, String> {
    let url = format!("{}/resource-templates", API_BASE);
    post_json::<ResourceTemplate, ResourceTemplate>(&url, template).await
}

pub async fn update_resource_template(uri_template: &str, template: &ResourceTemplate) -> Result<ResourceTemplate, String> {
    let encoded_uri = urlencoding_encode(uri_template);
    let url = format!("{}/resource-templates/{}", API_BASE, encoded_uri);
    put_json::<ResourceTemplate, ResourceTemplate>(&url, template).await
}

pub async fn delete_resource_template(uri_template: &str) -> Result<(), String> {
    let encoded_uri = urlencoding_encode(uri_template);
    let url = format!("{}/resource-templates/{}", API_BASE, encoded_uri);
    delete_request(&url).await
}

pub async fn test_resource_template(uri_template: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let encoded_uri = urlencoding_encode(uri_template);
    let url = format!("{}/resource-templates/{}/test", API_BASE, encoded_uri);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Tools
// ============================================================================

pub async fn list_tools() -> Result<Vec<Tool>, String> {
    let url = format!("{}/tools", API_BASE);
    fetch_json::<Vec<Tool>>(&url).await
}

pub async fn get_tool(name: &str) -> Result<Tool, String> {
    let url = format!("{}/tools/{}", API_BASE, name);
    fetch_json::<Tool>(&url).await
}

pub async fn create_tool(tool: &Tool) -> Result<Tool, String> {
    let url = format!("{}/tools", API_BASE);
    post_json::<Tool, Tool>(&url, tool).await
}

pub async fn update_tool(name: &str, tool: &Tool) -> Result<Tool, String> {
    let url = format!("{}/tools/{}", API_BASE, name);
    put_json::<Tool, Tool>(&url, tool).await
}

pub async fn delete_tool(name: &str) -> Result<(), String> {
    let url = format!("{}/tools/{}", API_BASE, name);
    delete_request(&url).await
}

pub async fn test_tool(name: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let url = format!("{}/tools/{}/test", API_BASE, name);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Prompts
// ============================================================================

pub async fn list_prompts() -> Result<Vec<Prompt>, String> {
    let url = format!("{}/prompts", API_BASE);
    fetch_json::<Vec<Prompt>>(&url).await
}

pub async fn get_prompt(name: &str) -> Result<Prompt, String> {
    let url = format!("{}/prompts/{}", API_BASE, name);
    fetch_json::<Prompt>(&url).await
}

pub async fn create_prompt(prompt: &Prompt) -> Result<Prompt, String> {
    let url = format!("{}/prompts", API_BASE);
    post_json::<Prompt, Prompt>(&url, prompt).await
}

pub async fn update_prompt(name: &str, prompt: &Prompt) -> Result<Prompt, String> {
    let url = format!("{}/prompts/{}", API_BASE, name);
    put_json::<Prompt, Prompt>(&url, prompt).await
}

pub async fn delete_prompt(name: &str) -> Result<(), String> {
    let url = format!("{}/prompts/{}", API_BASE, name);
    delete_request(&url).await
}

pub async fn test_prompt(name: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let url = format!("{}/prompts/{}/test", API_BASE, name);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Workflows
// ============================================================================

pub async fn list_workflows() -> Result<Vec<Workflow>, String> {
    let url = format!("{}/workflows", API_BASE);
    fetch_json::<Vec<Workflow>>(&url).await
}

pub async fn get_workflow(name: &str) -> Result<Workflow, String> {
    let url = format!("{}/workflows/{}", API_BASE, name);
    fetch_json::<Workflow>(&url).await
}

pub async fn create_workflow(workflow: &Workflow) -> Result<Workflow, String> {
    let url = format!("{}/workflows", API_BASE);
    post_json::<Workflow, Workflow>(&url, workflow).await
}

pub async fn update_workflow(name: &str, workflow: &Workflow) -> Result<Workflow, String> {
    let url = format!("{}/workflows/{}", API_BASE, name);
    put_json::<Workflow, Workflow>(&url, workflow).await
}

pub async fn delete_workflow(name: &str) -> Result<(), String> {
    let url = format!("{}/workflows/{}", API_BASE, name);
    delete_request(&url).await
}

pub async fn test_workflow(name: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let url = format!("{}/workflows/{}/test", API_BASE, name);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Agents
// ============================================================================

pub async fn list_agents() -> Result<Vec<Agent>, String> {
    let url = format!("{}/agents", API_BASE);
    fetch_json::<Vec<Agent>>(&url).await
}

pub async fn get_agent(name: &str) -> Result<Agent, String> {
    let url = format!("{}/agents/{}", API_BASE, name);
    fetch_json::<Agent>(&url).await
}

pub async fn create_agent(agent: &Agent) -> Result<Agent, String> {
    let url = format!("{}/agents", API_BASE);
    post_json::<Agent, Agent>(&url, agent).await
}

pub async fn update_agent(name: &str, agent: &Agent) -> Result<Agent, String> {
    let url = format!("{}/agents/{}", API_BASE, name);
    put_json::<Agent, Agent>(&url, agent).await
}

pub async fn delete_agent(name: &str) -> Result<(), String> {
    let url = format!("{}/agents/{}", API_BASE, name);
    delete_request(&url).await
}

pub async fn test_agent(name: &str, args: &serde_json::Value, session_id: Option<String>) -> Result<crate::types::TestResult, String> {
    let url = format!("{}/agents/{}/test", API_BASE, name);
    let req = crate::types::TestRequest {
        args: args.clone(),
        session_id,
    };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Orchestrations
// ============================================================================

pub async fn list_orchestrations() -> Result<Vec<Orchestration>, String> {
    let url = format!("{}/orchestrations", API_BASE);
    fetch_json::<Vec<Orchestration>>(&url).await
}

pub async fn get_orchestration(name: &str) -> Result<Orchestration, String> {
    let url = format!("{}/orchestrations/{}", API_BASE, name);
    fetch_json::<Orchestration>(&url).await
}

pub async fn create_orchestration(orchestration: &Orchestration) -> Result<Orchestration, String> {
    let url = format!("{}/orchestrations", API_BASE);
    post_json::<Orchestration, Orchestration>(&url, orchestration).await
}

pub async fn update_orchestration(name: &str, orchestration: &Orchestration) -> Result<Orchestration, String> {
    let url = format!("{}/orchestrations/{}", API_BASE, name);
    put_json::<Orchestration, Orchestration>(&url, orchestration).await
}

pub async fn delete_orchestration(name: &str) -> Result<(), String> {
    let url = format!("{}/orchestrations/{}", API_BASE, name);
    delete_request(&url).await
}

pub async fn test_orchestration(name: &str, args: &serde_json::Value) -> Result<crate::types::TestResult, String> {
    let url = format!("{}/orchestrations/{}/test", API_BASE, name);
    let req = crate::types::TestRequest { args: args.clone(), session_id: None };
    post_json::<crate::types::TestRequest, crate::types::TestResult>(&url, &req).await
}

// ============================================================================
// Schemas (Reusable JSON Schema Definitions)
// ============================================================================

pub async fn list_schemas() -> Result<Vec<Schema>, String> {
    let url = format!("{}/schemas", API_BASE);
    fetch_json::<Vec<Schema>>(&url).await
}

pub async fn get_schema(name: &str) -> Result<Schema, String> {
    let url = format!("{}/schemas/{}", API_BASE, name);
    fetch_json::<Schema>(&url).await
}

pub async fn create_schema(schema: &Schema) -> Result<Schema, String> {
    let url = format!("{}/schemas", API_BASE);
    post_json::<Schema, Schema>(&url, schema).await
}

pub async fn update_schema(name: &str, schema: &Schema) -> Result<Schema, String> {
    let url = format!("{}/schemas/{}", API_BASE, name);
    put_json::<Schema, Schema>(&url, schema).await
}

pub async fn delete_schema(name: &str) -> Result<(), String> {
    let url = format!("{}/schemas/{}", API_BASE, name);
    delete_request(&url).await
}

// ============================================================================
// State Management
// ============================================================================

pub async fn get_state() -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let url = format!("{}/state", API_BASE);
    fetch_json::<std::collections::HashMap<String, serde_json::Value>>(&url).await
}

pub async fn reset_state() -> Result<(), String> {
    let url = format!("{}/state", API_BASE);
    delete_request(&url).await
}

// ============================================================================
// Secrets Management
// ============================================================================

/// Status of a single secret key
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecretKeyStatus {
    pub key: String,
    pub label: String,
    pub description: String,
    pub is_set: bool,
    pub category: String,
}

/// Response showing which secrets are configured
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecretsStatusResponse {
    /// List of secret keys that have been set
    pub configured: Vec<String>,
    /// All known secret keys with their set status
    pub keys: Vec<SecretKeyStatus>,
}

/// List all secrets and their status (not values)
pub async fn list_secrets() -> Result<SecretsStatusResponse, String> {
    let url = format!("{}/secrets", API_BASE);
    fetch_json::<SecretsStatusResponse>(&url).await
}

/// Set a secret value
pub async fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let url = format!("{}/secrets/{}", API_BASE, key);
    #[derive(serde::Serialize)]
    struct SetSecretRequest {
        value: String,
    }
    let req = SetSecretRequest {
        value: value.to_string(),
    };
    post_empty(&url, &req).await
}

/// Delete a secret
pub async fn delete_secret(key: &str) -> Result<(), String> {
    let url = format!("{}/secrets/{}", API_BASE, key);
    delete_request(&url).await
}

/// Clear all secrets
pub async fn clear_secrets() -> Result<(), String> {
    let url = format!("{}/secrets", API_BASE);
    delete_request(&url).await
}

// ============================================================================
// LLM Models Discovery
// ============================================================================

/// Model info returned from LLM providers
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Fetch available models for a given LLM provider
pub async fn fetch_llm_models(provider: &str, base_url: Option<&str>, api_key_env: Option<&str>) -> Result<Vec<LlmModelInfo>, String> {
    let mut url = format!("{}/llm/models/{}", API_BASE, provider);

    let mut params = Vec::new();
    if let Some(base) = base_url {
        if !base.is_empty() {
            params.push(format!("base_url={}", urlencoding_encode(base)));
        }
    }
    if let Some(env) = api_key_env {
        if !env.is_empty() {
            params.push(format!("api_key_env={}", urlencoding_encode(env)));
        }
    }

    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    fetch_json::<Vec<LlmModelInfo>>(&url).await
}

// ============================================================================
// Helper functions
// ============================================================================

fn urlencoding_encode(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let response = Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let api_response: ApiResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.success {
        api_response.data.ok_or_else(|| "No data in response".to_string())
    } else {
        Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

async fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    url: &str,
    body: &T,
) -> Result<R, String> {
    let response = Request::post(url)
        .json(body)
        .map_err(|e| format!("Failed to serialize body: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let api_response: ApiResponse<R> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.success {
        api_response.data.ok_or_else(|| "No data in response".to_string())
    } else {
        Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

async fn put_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    url: &str,
    body: &T,
) -> Result<R, String> {
    let response = Request::put(url)
        .json(body)
        .map_err(|e| format!("Failed to serialize body: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let api_response: ApiResponse<R> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.success {
        api_response.data.ok_or_else(|| "No data in response".to_string())
    } else {
        Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

async fn delete_request(url: &str) -> Result<(), String> {
    let response = Request::delete(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let api_response: ApiResponse<()> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.success {
        Ok(())
    } else {
        Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

/// POST request that expects no data in response (just success/error)
async fn post_empty<T: serde::Serialize>(url: &str, body: &T) -> Result<(), String> {
    let response = Request::post(url)
        .json(body)
        .map_err(|e| format!("Failed to serialize body: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let api_response: ApiResponse<()> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.success {
        Ok(())
    } else {
        Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}
