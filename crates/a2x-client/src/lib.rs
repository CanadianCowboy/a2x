// a2x-client — Rust client SDK for connecting to A2X
// See plans/06-entity-gateway.md §7
//
// High-level client for connecting any Rust application to the A2X gateway.
// Supports HTTP/REST, WebSocket streaming, and webhook registration.

use reqwest::Client;
use serde::{Deserialize, Serialize};

// ── Client ────────────────────────────────────────────────────────────────

/// High-level client for connecting to an A2X gateway.
///
/// # Example
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use a2x_client::A2xClient;
///
/// let client = A2xClient::new("http://localhost:8778", "sk-my-api-key");
/// let result = client.execute("⟦Σ∞⟧⟬I:✕ ⟭").await?;
/// println!("Result: {:?}", result);
/// # Ok(())
/// # }
/// ```
pub struct A2xClient {
    /// Base URL of the gateway (e.g. "http://localhost:8778").
    gateway_url: String,
    /// API key for authentication.
    api_key: String,
    /// HTTP client.
    client: Client,
}

impl A2xClient {
    /// Create a new client connected to the given gateway.
    pub fn new(gateway_url: &str, api_key: &str) -> Self {
        A2xClient {
            gateway_url: gateway_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }

    /// Create a client with a custom reqwest client (e.g. with timeout config).
    pub fn with_client(gateway_url: &str, api_key: &str, client: Client) -> Self {
        A2xClient {
            gateway_url: gateway_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client,
        }
    }

    /// Execute a Σ∞ program and wait for the result.
    pub async fn execute(&self, program: &str) -> Result<ExecuteResult, ClientError> {
        let url = format!("{}/a2x/execute", self.gateway_url);
        let body = ExecuteRequest {
            program: program.to_string(),
            format: "sigma".into(),
            timeout_ms: 5000,
        };

        let resp = self
            .client
            .post(&url)
            .header("X-A2X-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ServerError { status, body: text });
        }

        let result: ExecuteResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))?;

        Ok(ExecuteResult {
            result: result.result,
            execution_time_ms: result.execution_time_ms,
            status: result.status,
        })
    }

    /// List all connected entities and agents.
    pub async fn list_entities(&self) -> Result<Vec<EntityInfo>, ClientError> {
        let url = format!("{}/a2x/entities", self.gateway_url);

        let resp = self
            .client
            .get(&url)
            .header("X-A2X-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ServerError { status, body: text });
        }

        let entities: Vec<EntityInfo> = resp
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))?;

        Ok(entities)
    }

    /// Probe an agent's internal state.
    pub async fn probe_agent(&self, agent_id: &str) -> Result<ProbeResult, ClientError> {
        let url = format!("{}/a2x/probe/{}", self.gateway_url, agent_id);

        let resp = self
            .client
            .get(&url)
            .header("X-A2X-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ServerError { status, body: text });
        }

        let result: ProbeResult = resp
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))?;

        Ok(result)
    }

    /// Register a webhook for async result callbacks.
    pub async fn register_webhook(
        &self,
        url: &str,
        filter_correlation_ids: Option<Vec<u64>>,
    ) -> Result<String, ClientError> {
        let endpoint = format!("{}/a2x/webhook", self.gateway_url);
        let body = WebhookRegisterRequest {
            url: url.to_string(),
            filter_correlation_ids,
        };

        let resp = self
            .client
            .post(&endpoint)
            .header("X-A2X-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(ClientError::ServerError { status, body: text });
        }

        let result: WebhookRegisterResponse = resp
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))?;

        Ok(result.webhook_id)
    }
}

// ── Request / Response types ──────────────────────────────────────────────

#[derive(Serialize)]
struct ExecuteRequest {
    program: String,
    format: String,
    timeout_ms: u64,
}

#[derive(Deserialize, Debug)]
struct ExecuteResponse {
    result: String,
    execution_time_ms: u64,
    status: String,
}

#[derive(Serialize)]
struct WebhookRegisterRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_correlation_ids: Option<Vec<u64>>,
}

#[derive(Deserialize)]
struct WebhookRegisterResponse {
    webhook_id: String,
}

// ── Public result types ───────────────────────────────────────────────────

/// Result of executing a Σ∞ program.
#[derive(Debug, Clone)]
pub struct ExecuteResult {
    /// The result Σ∞ program text.
    pub result: String,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Status: "completed", "error", "timeout".
    pub status: String,
}

/// Information about a connected entity.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityInfo {
    pub id: String,
    pub entity_type: String,
    pub display_name: String,
    pub capabilities: Vec<String>,
}

/// Result of probing an agent.
#[derive(Debug, Clone, Deserialize)]
pub struct ProbeResult {
    pub agent_id: String,
    pub state: String,
    pub ip: Option<usize>,
    pub world_graph_size: usize,
    pub memory_trace_length: usize,
}

// ── Error type ────────────────────────────────────────────────────────────

/// Error from the client SDK.
#[derive(Debug)]
pub enum ClientError {
    /// Network error (connection refused, timeout, etc.).
    Network(String),
    /// Server returned an error status code.
    ServerError { status: u16, body: String },
    /// Failed to parse the response.
    ParseError(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Network(msg) => write!(f, "network error: {}", msg),
            ClientError::ServerError { status, body } => {
                write!(f, "server error ({}): {}", status, body)
            }
            ClientError::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for ClientError {}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = A2xClient::new("http://localhost:8778", "sk-test");
        assert_eq!(client.gateway_url, "http://localhost:8778");
        assert_eq!(client.api_key, "sk-test");
    }

    #[test]
    fn test_client_trailing_slash() {
        let client = A2xClient::new("http://localhost:8778/", "sk-test");
        assert_eq!(client.gateway_url, "http://localhost:8778");
    }

    #[test]
    fn test_execute_result_display() {
        let result = ExecuteResult {
            result: "⟦Σ∞⟧⟬I:✕⟭".into(),
            execution_time_ms: 42,
            status: "completed".into(),
        };
        assert_eq!(result.status, "completed");
        assert_eq!(result.execution_time_ms, 42);
    }

    #[test]
    fn test_client_error_display() {
        let err = ClientError::Network("connection refused".into());
        assert!(format!("{}", err).contains("connection refused"));

        let err = ClientError::ServerError {
            status: 404,
            body: "not found".into(),
        };
        assert!(format!("{}", err).contains("404"));

        let err = ClientError::ParseError("bad json".into());
        assert!(format!("{}", err).contains("parse error"));
    }

    #[test]
    fn test_entity_info_deserialize() {
        let json = r#"{"id":"e-1","entity_type":"Application","display_name":"App","capabilities":["execute"]}"#;
        let info: EntityInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "e-1");
        assert_eq!(info.capabilities, vec!["execute"]);
    }

    #[test]
    fn test_probe_result_deserialize() {
        let json = r#"{"agent_id":"ccs-1","state":"idle","ip":3,"world_graph_size":10,"memory_trace_length":3}"#;
        let result: ProbeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.agent_id, "ccs-1");
        assert_eq!(result.ip, Some(3));
    }
}
