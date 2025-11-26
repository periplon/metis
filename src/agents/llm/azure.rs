//! Azure OpenAI LLM Provider with streaming support

use async_trait::async_trait;
use std::env;

use super::{
    CompletionRequest, CompletionResponse, LlmProvider, LlmStream,
    openai::OpenAiProvider,
};
use crate::adapters::secrets::SharedSecretsStore;
use crate::agents::config::LlmProviderConfig;
use crate::agents::error::{LlmError, LlmResult};

/// Azure OpenAI LLM Provider
/// Uses the same API format as OpenAI but with Azure-specific endpoints
pub struct AzureOpenAiProvider {
    inner: OpenAiProvider,
}

impl AzureOpenAiProvider {
    /// Create a new Azure OpenAI provider from configuration
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        // Azure requires specific endpoint format:
        // https://{resource-name}.openai.azure.com/openai/deployments/{deployment-name}/chat/completions?api-version={api-version}

        let api_key = if let Some(env_var) = &config.api_key_env {
            env::var(env_var).map_err(|_| {
                LlmError::Authentication(format!(
                    "Environment variable {} not set",
                    env_var
                ))
            })?
        } else {
            env::var("AZURE_OPENAI_API_KEY").map_err(|_| {
                LlmError::Authentication("AZURE_OPENAI_API_KEY environment variable not set".to_string())
            })?
        };

        let base_url = config.base_url.clone().ok_or_else(|| {
            LlmError::InvalidRequest(
                "Azure OpenAI requires base_url to be set (e.g., https://your-resource.openai.azure.com)".to_string()
            )
        })?;

        // Create a modified config for the inner OpenAI provider
        let mut inner_config = config.clone();
        inner_config.api_key_env = Some("__AZURE_KEY__".to_string());

        // Set the environment variable temporarily
        env::set_var("__AZURE_KEY__", &api_key);

        // Construct Azure-specific URL
        let _api_version = "2024-02-15-preview"; // Default API version, used in URL construction
        let deployment = &config.model;
        inner_config.base_url = Some(format!(
            "{}/openai/deployments/{}",
            base_url.trim_end_matches('/'),
            deployment
        ));

        let inner = OpenAiProvider::new(&inner_config)?;

        Ok(Self { inner })
    }

    /// Create a new Azure OpenAI provider using secrets store for API key
    pub async fn new_with_secrets(
        config: &LlmProviderConfig,
        secrets: SharedSecretsStore,
    ) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or("AZURE_OPENAI_API_KEY");

        let api_key = secrets.get_or_env(env_var).await.ok_or_else(|| {
            LlmError::Authentication(format!(
                "API key not found in secrets store or environment variable {}",
                env_var
            ))
        })?;

        let base_url = config.base_url.clone().ok_or_else(|| {
            LlmError::InvalidRequest(
                "Azure OpenAI requires base_url to be set (e.g., https://your-resource.openai.azure.com)".to_string()
            )
        })?;

        // Create a modified config for the inner OpenAI provider
        let mut inner_config = config.clone();
        inner_config.api_key_env = Some("__AZURE_KEY__".to_string());

        // Set the environment variable temporarily
        env::set_var("__AZURE_KEY__", &api_key);

        // Construct Azure-specific URL
        let deployment = &config.model;
        inner_config.base_url = Some(format!(
            "{}/openai/deployments/{}",
            base_url.trim_end_matches('/'),
            deployment
        ));

        let inner = OpenAiProvider::new(&inner_config)?;

        Ok(Self { inner })
    }
}

#[async_trait]
impl LlmProvider for AzureOpenAiProvider {
    fn name(&self) -> &str {
        "azure-openai"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        // Azure uses the same API format as OpenAI
        self.inner.complete(request).await
    }

    fn complete_stream(&self, request: CompletionRequest) -> LlmStream {
        self.inner.complete_stream(request)
    }

    fn count_tokens(&self, text: &str) -> u32 {
        self.inner.count_tokens(text)
    }

    fn context_window(&self) -> u32 {
        self.inner.context_window()
    }

    fn max_output_tokens(&self) -> u32 {
        self.inner.max_output_tokens()
    }
}
