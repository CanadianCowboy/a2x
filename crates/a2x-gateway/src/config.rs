// See plans/06-entity-gateway.md §8
// Gateway configuration (TOML).

use serde::Deserialize;

/// Top-level gateway configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct GatewayConfig {
    /// Gateway bind address for the main control plane.
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// HTTP listener configuration.
    #[serde(default)]
    pub http: HttpConfig,

    /// WebSocket listener configuration.
    #[serde(default)]
    pub websocket: WebSocketConfig,

    /// TCP listener configuration.
    #[serde(default)]
    pub tcp: TcpConfig,

    /// stdin/stdout listener configuration.
    #[serde(default)]
    pub stdio: StdioConfig,

    /// Authentication configuration.
    #[serde(default)]
    pub auth: AuthConfig,

    /// Webhook configuration.
    #[serde(default)]
    pub webhook: WebhookConfig,

    /// Chat agent LLM backend configuration.
    #[serde(default)]
    pub chat_backend: ChatBackendConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_http_port")]
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            enabled: true,
            port: 8778,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct WebSocketConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ws_port")]
    pub port: u16,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        WebSocketConfig {
            enabled: true,
            port: 8779,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TcpConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tcp_port")]
    pub port: u16,
}

impl Default for TcpConfig {
    fn default() -> Self {
        TcpConfig {
            enabled: true,
            port: 8780,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct StdioConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for StdioConfig {
    fn default() -> Self {
        StdioConfig { enabled: true }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    /// Authentication mode: "api_key", "none", or "local".
    #[serde(default = "default_auth_mode")]
    pub mode: String,
    /// Pre-registered API keys (key → entity_id).
    #[serde(default)]
    pub api_keys: Vec<ApiKeyEntry>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            mode: "local".into(),
            api_keys: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiKeyEntry {
    pub key: String,
    pub entity_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WebhookConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Timeout for webhook delivery in milliseconds.
    #[serde(default = "default_webhook_timeout")]
    pub timeout_ms: u64,
    /// Maximum retry attempts for failed webhooks.
    #[serde(default = "default_webhook_retries")]
    pub max_retries: u32,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        WebhookConfig {
            enabled: true,
            timeout_ms: 10_000,
            max_retries: 3,
        }
    }
}

/// LLM backend configuration for the chat agent.
#[derive(Clone, Debug, Deserialize)]
pub struct ChatBackendConfig {
    /// Backend type: "ollama", "openai", or "none".
    #[serde(default = "default_backend_type")]
    pub backend_type: String,

    /// Model name (e.g. "codellama:7b", "llama3", "gpt-4o").
    #[serde(default = "default_chat_model")]
    pub model: String,

    /// API base URL (Ollama default: http://localhost:11434/v1).
    #[serde(default = "default_ollama_url")]
    pub api_url: String,

    /// API key (empty for Ollama, required for OpenAI).
    #[serde(default)]
    pub api_key: String,

    /// Maximum response tokens.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Temperature (0.0 = deterministic, 1.0 = creative).
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum context window tokens (for pruning old messages).
    /// Default 32768 — matches qwen2.5 / mistral 32K context windows.
    /// Set lower for smaller models like deepseek-coder-v2 (2048).
    #[serde(default = "default_context_tokens")]
    pub max_context_tokens: u32,
}

impl Default for ChatBackendConfig {
    fn default() -> Self {
        ChatBackendConfig {
            backend_type: default_backend_type(),
            model: default_chat_model(),
            api_url: default_ollama_url(),
            api_key: String::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            max_context_tokens: default_context_tokens(),
        }
    }
}

fn default_backend_type() -> String {
    "none".into()
}
fn default_chat_model() -> String {
    "codellama:7b".into()
}
fn default_ollama_url() -> String {
    "http://localhost:11434/v1".into()
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.2
}
fn default_context_tokens() -> u32 {
    32768
}

// Serde defaults
fn default_bind_address() -> String {
    "0.0.0.0:8777".into()
}
fn default_true() -> bool {
    true
}
fn default_http_port() -> u16 {
    8778
}
fn default_ws_port() -> u16 {
    8779
}
fn default_tcp_port() -> u16 {
    8780
}
fn default_auth_mode() -> String {
    "local".into()
}
fn default_webhook_timeout() -> u64 {
    10_000
}
fn default_webhook_retries() -> u32 {
    3
}

impl Default for GatewayConfig {
    fn default() -> Self {
        GatewayConfig {
            bind_address: default_bind_address(),
            http: HttpConfig::default(),
            websocket: WebSocketConfig::default(),
            tcp: TcpConfig::default(),
            stdio: StdioConfig::default(),
            auth: AuthConfig::default(),
            webhook: WebhookConfig::default(),
            chat_backend: ChatBackendConfig::default(),
        }
    }
}

impl GatewayConfig {
    /// Parse a TOML config string.
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = GatewayConfig::default();
        assert_eq!(cfg.bind_address, "0.0.0.0:8777");
        assert!(cfg.http.enabled);
        assert_eq!(cfg.http.port, 8778);
        assert!(cfg.websocket.enabled);
        assert_eq!(cfg.websocket.port, 8779);
        assert!(cfg.tcp.enabled);
        assert_eq!(cfg.tcp.port, 8780);
        assert!(cfg.stdio.enabled);
        assert_eq!(cfg.auth.mode, "local");
        assert!(cfg.webhook.enabled);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
bind_address = "127.0.0.1:9000"

[http]
enabled = true
port = 9001

[auth]
mode = "api_key"

[[auth.api_keys]]
key = "sk-test"
entity_id = "app-1"
"#;
        let cfg = GatewayConfig::from_toml(toml_str).unwrap();
        assert_eq!(cfg.bind_address, "127.0.0.1:9000");
        assert_eq!(cfg.http.port, 9001);
        assert_eq!(cfg.auth.mode, "api_key");
        assert_eq!(cfg.auth.api_keys.len(), 1);
        assert_eq!(cfg.auth.api_keys[0].key, "sk-test");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = "";
        let cfg = GatewayConfig::from_toml(toml_str).unwrap();
        // All defaults should kick in
        assert_eq!(cfg.bind_address, "0.0.0.0:8777");
    }
}
