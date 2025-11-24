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

/// Save config to disk (metis.toml)
pub async fn save_config_to_disk() -> Result<(), String> {
    let url = format!("{}/config/save-disk", API_BASE);
    post_empty(&url, &()).await
}

/// Save config to S3
pub async fn save_config_to_s3() -> Result<(), String> {
    let url = format!("{}/config/save-s3", API_BASE);
    post_empty(&url, &()).await
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
// Helper functions
// ============================================================================

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
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
