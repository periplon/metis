use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::auth::{AuthConfig, AuthContext, AuthMode};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    #[serde(default)]
    roles: Vec<String>,
}

pub struct AuthMiddleware {
    config: Arc<AuthConfig>,
}

impl AuthMiddleware {
    pub fn new(config: Arc<AuthConfig>) -> Self {
        Self { config }
    }

    pub async fn authenticate(
        &self,
        headers: &HeaderMap,
    ) -> Result<AuthContext, AuthError> {
        if !self.config.enabled {
            return Ok(AuthContext::default());
        }

        match self.config.mode {
            AuthMode::None => Ok(AuthContext::default()),
            AuthMode::ApiKey => self.validate_api_key(headers),
            AuthMode::BearerToken => self.validate_bearer_token(headers),
            _ => Err(AuthError::UnsupportedAuthMode),
        }
    }

    fn validate_api_key(&self, headers: &HeaderMap) -> Result<AuthContext, AuthError> {
        let api_key = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::MissingCredentials)?;

        let valid_keys = self.config.api_keys.as_ref()
            .ok_or(AuthError::ConfigurationError)?;

        if valid_keys.contains(&api_key.to_string()) {
            Ok(AuthContext {
                authenticated: true,
                user_id: Some(api_key.to_string()),
                roles: vec!["user".to_string()],
            })
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    fn validate_bearer_token(&self, headers: &HeaderMap) -> Result<AuthContext, AuthError> {
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError::MissingCredentials)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidCredentials);
        }

        let token = &auth_header[7..];
        let secret = self.config.jwt_secret.as_ref()
            .ok_or(AuthError::ConfigurationError)?;

        let algorithm = match self.config.jwt_algorithm.as_deref() {
            Some("HS256") => Algorithm::HS256,
            Some("HS384") => Algorithm::HS384,
            Some("HS512") => Algorithm::HS512,
            _ => Algorithm::HS256,
        };

        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ).map_err(|_| AuthError::InvalidCredentials)?;

        Ok(AuthContext {
            authenticated: true,
            user_id: Some(token_data.claims.sub),
            roles: token_data.claims.roles,
        })
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingCredentials,
    InvalidCredentials,
    UnsupportedAuthMode,
    ConfigurationError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingCredentials => (StatusCode::UNAUTHORIZED, "Missing credentials"),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AuthError::UnsupportedAuthMode => (StatusCode::INTERNAL_SERVER_ERROR, "Unsupported auth mode"),
            AuthError::ConfigurationError => (StatusCode::INTERNAL_SERVER_ERROR, "Auth configuration error"),
        };

        (status, message).into_response()
    }
}

pub async fn auth_middleware(
    auth: Arc<AuthMiddleware>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let auth_context = auth.authenticate(request.headers()).await?;
    
    // Store auth context in request extensions
    request.extensions_mut().insert(auth_context);
    
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[tokio::test]
    async fn test_api_key_auth_success() {
        let config = Arc::new(AuthConfig {
            enabled: true,
            mode: AuthMode::ApiKey,
            api_keys: Some(vec!["test-key-123".to_string()]),
            jwt_secret: None,
            jwt_algorithm: None,
        });

        let middleware = AuthMiddleware::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("test-key-123"));

        let result = middleware.authenticate(&headers).await;
        assert!(result.is_ok());
        assert!(result.unwrap().authenticated);
    }

    #[tokio::test]
    async fn test_api_key_auth_failure() {
        let config = Arc::new(AuthConfig {
            enabled: true,
            mode: AuthMode::ApiKey,
            api_keys: Some(vec!["test-key-123".to_string()]),
            jwt_secret: None,
            jwt_algorithm: None,
        });

        let middleware = AuthMiddleware::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("wrong-key"));

        let result = middleware.authenticate(&headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disabled_auth() {
        let config = Arc::new(AuthConfig {
            enabled: false,
            mode: AuthMode::None,
            api_keys: None,
            jwt_secret: None,
            jwt_algorithm: None,
        });

        let middleware = AuthMiddleware::new(config);
        let headers = HeaderMap::new();

        let result = middleware.authenticate(&headers).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().authenticated);
    }
}
