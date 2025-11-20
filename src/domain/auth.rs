use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMode {
    None,
    ApiKey,
    BearerToken,
    BasicAuth,
    OAuth2,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub mode: AuthMode,
    pub api_keys: Option<Vec<String>>,
    pub jwt_secret: Option<String>,
    pub jwt_algorithm: Option<String>,
    pub basic_users: Option<std::collections::HashMap<String, String>>,
    pub jwks_url: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AuthMode::None,
            api_keys: None,
            jwt_secret: None,
            jwt_algorithm: Some("HS256".to_string()),
            basic_users: None,
            jwks_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthContext {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub roles: Vec<String>,
}
