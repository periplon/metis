use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
    pub kid: String,
    pub kty: String,
    pub alg: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Clone)]
pub struct JwksClient {
    url: String,
    #[allow(clippy::type_complexity)]
    cache: Arc<RwLock<Option<(HashMap<String, Jwk>, Instant)>>>,
    ttl: Duration,
}

impl JwksClient {
    pub fn new(url: String) -> Self {
        Self {
            url,
            cache: Arc::new(RwLock::new(None)),
            ttl: Duration::from_secs(900), // 15 minutes
        }
    }

    pub async fn get_key(&self, kid: &str) -> Result<Jwk, anyhow::Error> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some((keys, timestamp)) = &*cache {
                if timestamp.elapsed() < self.ttl {
                    if let Some(key) = keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Refresh cache
        self.refresh().await?;

        // Try cache again
        let cache = self.cache.read().await;
        if let Some((keys, _)) = &*cache {
            if let Some(key) = keys.get(kid) {
                return Ok(key.clone());
            }
        }

        Err(anyhow::anyhow!("Key ID {} not found in JWKS", kid))
    }

    async fn refresh(&self) -> Result<(), anyhow::Error> {
        let jwks: Jwks = reqwest::get(&self.url).await?.json().await?;
        
        let mut key_map = HashMap::new();
        for key in jwks.keys {
            key_map.insert(key.kid.clone(), key);
        }

        let mut cache = self.cache.write().await;
        *cache = Some((key_map, Instant::now()));
        
        Ok(())
    }
}
