//! HTTP Client for Load Testing
//!
//! High-performance HTTP client with connection pooling, retries,
//! and comprehensive error handling for load testing.

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client configuration
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the service
    pub base_url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum idle connections per host
    pub pool_idle_per_host: usize,
    /// Maximum retries for transient errors
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            pool_idle_per_host: 100,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// Response wrapper for API calls
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// String value response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StringValue {
    pub value: String,
    pub ttl: Option<i64>,
}

/// Set string request
#[derive(Debug, Serialize)]
pub struct SetStringRequest {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
}

/// MGet request
#[derive(Debug, Serialize)]
pub struct MGetRequest {
    pub keys: Vec<String>,
}

/// MGet response
#[derive(Debug, Deserialize)]
pub struct MGetResponse {
    pub values: std::collections::HashMap<String, Option<String>>,
}

/// MSet request
#[derive(Debug, Serialize)]
pub struct MSetRequest {
    pub pairs: std::collections::HashMap<String, String>,
}

/// Increment request
#[derive(Debug, Serialize)]
pub struct IncrementRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<i64>,
}

/// Increment response
#[derive(Debug, Deserialize)]
pub struct IncrementResponse {
    pub value: i64,
}

/// Load test HTTP client
#[derive(Clone)]
pub struct LoadTestClient {
    client: Client,
    config: ClientConfig,
}

impl LoadTestClient {
    /// Create new client with default config
    pub fn new(base_url: &str) -> Result<Self> {
        let config = ClientConfig {
            base_url: base_url.to_string(),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create client with custom config
    pub fn with_config(config: ClientConfig) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(config.pool_idle_per_host)
            .build()?;

        Ok(Self { client, config })
    }

    /// Get base URL
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Health check - tries both /health and /api/v1/health endpoints
    pub async fn health(&self) -> Result<bool> {
        // Try /health first (Rust service)
        let url = format!("{}/health", self.config.base_url);
        if let Ok(resp) = self.client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(true);
            }
        }

        // Try /api/v1/health (Go service)
        let url = format!("{}/api/v1/health", self.config.base_url);
        if let Ok(resp) = self.client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Set a string value
    pub async fn set_string(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<()> {
        let url = format!("{}/api/v1/strings/{}", self.config.base_url, key);
        let body = SetStringRequest {
            value: value.to_string(),
            ttl,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("SET failed: {}", resp.status()))
        }
    }

    /// Get a string value
    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        let url = format!("{}/api/v1/strings/{}", self.config.base_url, key);
        let resp = self.client.get(&url).send().await?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<StringValue> = resp.json().await?;
                Ok(api_resp.data.map(|v| v.value))
            }
            StatusCode::NOT_FOUND => Ok(None),
            status => Err(anyhow!("GET failed: {}", status)),
        }
    }

    /// Delete a string value (GETDEL)
    #[allow(dead_code)]
    pub async fn delete_string(&self, key: &str) -> Result<Option<String>> {
        let url = format!("{}/api/v1/strings/{}", self.config.base_url, key);
        let resp = self.client.delete(&url).send().await?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<StringValue> = resp.json().await?;
                Ok(api_resp.data.map(|v| v.value))
            }
            StatusCode::NOT_FOUND => Ok(None),
            status => Err(anyhow!("DELETE failed: {}", status)),
        }
    }

    /// Multi-get strings
    pub async fn mget(&self, keys: Vec<String>) -> Result<std::collections::HashMap<String, Option<String>>> {
        let url = format!("{}/api/v1/strings/mget", self.config.base_url);
        let body = MGetRequest { keys };

        let resp = self.client.post(&url).json(&body).send().await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<MGetResponse> = resp.json().await?;
            Ok(api_resp.data.map_or(std::collections::HashMap::new(), |r| r.values))
        } else {
            Err(anyhow!("MGET failed: {}", resp.status()))
        }
    }

    /// Multi-set strings
    pub async fn mset(&self, pairs: std::collections::HashMap<String, String>) -> Result<()> {
        let url = format!("{}/api/v1/strings/mset", self.config.base_url);
        let body = MSetRequest { pairs };

        let resp = self.client.post(&url).json(&body).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("MSET failed: {}", resp.status()))
        }
    }

    /// Increment a counter
    pub async fn incr(&self, key: &str, delta: Option<i64>) -> Result<i64> {
        let url = format!("{}/api/v1/strings/{}/incr", self.config.base_url, key);
        let body = IncrementRequest { delta };

        let resp = self.client.patch(&url).json(&body).send().await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<IncrementResponse> = resp.json().await?;
            Ok(api_resp.data.map_or(0, |r| r.value))
        } else {
            Err(anyhow!("INCR failed: {}", resp.status()))
        }
    }

    /// Decrement a counter
    #[allow(dead_code)]
    pub async fn decr(&self, key: &str, delta: Option<i64>) -> Result<i64> {
        let url = format!("{}/api/v1/strings/{}/decr", self.config.base_url, key);
        let body = IncrementRequest { delta };

        let resp = self.client.patch(&url).json(&body).send().await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<IncrementResponse> = resp.json().await?;
            Ok(api_resp.data.map_or(0, |r| r.value))
        } else {
            Err(anyhow!("DECR failed: {}", resp.status()))
        }
    }

    /// Append to a string
    #[allow(dead_code)]
    pub async fn append(&self, key: &str, value: &str) -> Result<u64> {
        let url = format!("{}/api/v1/strings/{}/append", self.config.base_url, key);

        #[derive(Serialize)]
        struct AppendRequest {
            value: String,
        }

        #[derive(Deserialize)]
        struct AppendResponse {
            length: u64,
        }

        let resp = self
            .client
            .patch(&url)
            .json(&AppendRequest { value: value.to_string() })
            .send()
            .await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<AppendResponse> = resp.json().await?;
            Ok(api_resp.data.map_or(0, |r| r.length))
        } else {
            Err(anyhow!("APPEND failed: {}", resp.status()))
        }
    }

    /// Get string length
    #[allow(dead_code)]
    pub async fn strlen(&self, key: &str) -> Result<u64> {
        let url = format!("{}/api/v1/strings/{}/length", self.config.base_url, key);

        #[derive(Deserialize)]
        struct StrLenResponse {
            length: u64,
        }

        let resp = self.client.get(&url).send().await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<StrLenResponse> = resp.json().await?;
            Ok(api_resp.data.map_or(0, |r| r.length))
        } else {
            Err(anyhow!("STRLEN failed: {}", resp.status()))
        }
    }

    /// Execute a request with retry logic
    #[allow(dead_code)]
    pub async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(self.config.retry_delay * (attempt + 1)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Unknown error")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.pool_idle_per_host, 100);
    }

    #[test]
    fn test_client_creation() {
        let client = LoadTestClient::new("http://localhost:8080");
        assert!(client.is_ok());
    }
}
