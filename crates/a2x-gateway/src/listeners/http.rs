// See plans/06-entity-gateway.md §5 (HTTP/REST Listener)
//
// HTTP endpoints:
//   POST /a2x/execute        — Execute a Σ∞/Ω program
//   GET  /a2x/entities        — List connected entities/agents
//   GET  /a2x/entities/:id    — Get entity/agent details
//   GET  /a2x/probe/:agent_id — Probe agent state
//   POST /a2x/webhook         — Register a webhook callback

use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::gateway::GatewayState;
use crate::listeners::{ProtocolListener, ProtocolListenerType};
use crate::webhook::WebhookEntry;

// ── Request / Response types ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExecuteRequest {
    /// Σ∞ program source text.
    pub program: String,
    /// Format: "sigma" or "omega".
    #[serde(default = "default_format")]
    pub format: String,
    /// Timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_format() -> String {
    "sigma".into()
}
fn default_timeout() -> u64 {
    5000
}

#[derive(Serialize)]
pub struct ExecuteResponse {
    /// Result Σ∞ program text.
    pub result: String,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Status: "completed", "error", "timeout".
    pub status: String,
}

#[derive(Serialize)]
pub struct EntityResponse {
    pub id: String,
    pub entity_type: String,
    pub display_name: String,
    pub capabilities: Vec<String>,
}

#[derive(Serialize)]
pub struct ProbeResponse {
    pub agent_id: String,
    pub state: String,
    pub ip: Option<usize>,
    pub world_graph_size: usize,
    pub memory_trace_length: usize,
}

#[derive(Deserialize)]
pub struct WebhookRegisterRequest {
    pub url: String,
    #[serde(default)]
    pub filter_correlation_ids: Option<Vec<u64>>,
}

#[derive(Serialize)]
pub struct WebhookRegisterResponse {
    pub webhook_id: String,
}

/// Auth query parameters.
#[derive(Deserialize)]
pub struct AuthQuery {
    #[serde(default)]
    pub api_key: Option<String>,
}

// ── Shared state for axum handlers ────────────────────────────────────────

pub struct HttpGatewayState {
    /// Shared gateway state (entity registry, bus, etc.).
    pub gateway: Arc<Mutex<GatewayState>>,
}

// ── HTTP handlers ─────────────────────────────────────────────────────────

/// POST /a2x/execute — Execute a Σ∞ program.
async fn handle_execute(
    State(state): State<Arc<HttpGatewayState>>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start = std::time::Instant::now();

    // Parse the program
    let program = a2x_sigma::parse_program(&req.program).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("parse error: {}", e)})),
        )
    })?;

    // Execute via the gateway
    let result = {
        let gw = state.gateway.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("lock error: {}", e)})),
            )
        })?;
        gw.execute_program(&program).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("execution error: {}", e)})),
            )
        })?
    };

    let elapsed = start.elapsed().as_millis() as u64;
    let result_str = if result.is_empty() {
        String::new()
    } else {
        result
            .instructions
            .iter()
            .map(a2x_sigma::serialize_packet)
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(Json(ExecuteResponse {
        result: result_str,
        execution_time_ms: elapsed,
        status: "completed".into(),
    }))
}

/// GET /a2x/entities — List connected entities.
async fn handle_list_entities(
    State(state): State<Arc<HttpGatewayState>>,
) -> Result<Json<Vec<EntityResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let gw = state.gateway.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("lock error: {}", e)})),
        )
    })?;

    let entities: Vec<EntityResponse> = gw
        .list_entities()
        .iter()
        .map(|info| EntityResponse {
            id: info.id.to_string(),
            entity_type: format!("{:?}", info.entity_type),
            display_name: info.display_name.clone(),
            capabilities: info.capabilities.iter().map(|c| c.to_string()).collect(),
        })
        .collect();

    Ok(Json(entities))
}

/// GET /a2x/entities/:id — Get entity details.
async fn handle_get_entity(
    State(state): State<Arc<HttpGatewayState>>,
    Path(entity_id): Path<String>,
) -> Result<Json<EntityResponse>, (StatusCode, Json<serde_json::Value>)> {
    let gw = state.gateway.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("lock error: {}", e)})),
        )
    })?;

    let eid = EntityId::new(&entity_id);
    let entities = gw.list_entities();
    let info = entities.iter().find(|i| i.id == eid).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("entity '{}' not found", entity_id)})),
        )
    })?;

    Ok(Json(EntityResponse {
        id: info.id.to_string(),
        entity_type: format!("{:?}", info.entity_type),
        display_name: info.display_name.clone(),
        capabilities: info.capabilities.iter().map(|c| c.to_string()).collect(),
    }))
}

/// GET /a2x/probe/:agent_id — Probe an agent's state.
async fn handle_probe(
    State(state): State<Arc<HttpGatewayState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<ProbeResponse>, (StatusCode, Json<serde_json::Value>)> {
    let gw = state.gateway.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("lock error: {}", e)})),
        )
    })?;

    let snapshot = gw.probe_agent(&agent_id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("{}", e)})),
        )
    })?;

    Ok(Json(ProbeResponse {
        agent_id: snapshot.agent_id.to_string(),
        state: snapshot.state,
        ip: snapshot.ip,
        world_graph_size: snapshot.world_graph_size,
        memory_trace_length: snapshot.memory_trace_length,
    }))
}

/// POST /a2x/webhook — Register a webhook callback.
async fn handle_register_webhook(
    State(state): State<Arc<HttpGatewayState>>,
    Json(req): Json<WebhookRegisterRequest>,
) -> Result<Json<WebhookRegisterResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut gw = state.gateway.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("lock error: {}", e)})),
        )
    })?;

    // Register with a default entity — in production, the entity would
    // be identified by the auth header.
    let webhook_id = gw.webhooks.register(WebhookEntry {
        entity_id: EntityId::new("http-entity"),
        url: req.url,
        filter_correlation_ids: req.filter_correlation_ids,
    });

    Ok(Json(WebhookRegisterResponse { webhook_id }))
}

// ── HTTP Protocol Listener ────────────────────────────────────────────────

/// HTTP/REST protocol listener.
///
/// Binds to a TCP address and serves the A2X REST API via axum.
pub struct HttpListener {
    bind_address: String,
    #[allow(dead_code)]
    gateway_state: Arc<HttpGatewayState>,
    running: bool,
}

impl HttpListener {
    pub fn new(bind_address: impl Into<String>, gateway_state: Arc<HttpGatewayState>) -> Self {
        HttpListener {
            bind_address: bind_address.into(),
            gateway_state,
            running: false,
        }
    }

    /// Build the axum router with all A2X endpoints.
    #[allow(dead_code)]
    pub fn router(state: Arc<HttpGatewayState>) -> Router {
        Router::new()
            .route("/a2x/execute", post(handle_execute))
            .route("/a2x/entities", get(handle_list_entities))
            .route("/a2x/entities/{entity_id}", get(handle_get_entity))
            .route("/a2x/probe/{agent_id}", get(handle_probe))
            .route("/a2x/webhook", post(handle_register_webhook))
            .with_state(state)
    }
}

impl ProtocolListener for HttpListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::Http
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        // Note: In a full async implementation, this would spawn a tokio task.
        // For now, we mark as running and provide the router for integration.
        self.running = true;
        tracing::info!("HTTP listener started on {}", self.bind_address);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        self.running = false;
        tracing::info!("HTTP listener stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn bound_address(&self) -> Option<String> {
        if self.running {
            Some(self.bind_address.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_request_defaults() {
        let req: ExecuteRequest = serde_json::from_str(r#"{"program": "⟦Σ∞⟧⟬I:✕⟭"}"#).unwrap();
        assert_eq!(req.format, "sigma");
        assert_eq!(req.timeout_ms, 5000);
    }

    #[test]
    fn test_entity_response_serialization() {
        let resp = EntityResponse {
            id: "e-1".into(),
            entity_type: "Application".into(),
            display_name: "Test App".into(),
            capabilities: vec!["execute".into()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("e-1"));
        assert!(json.contains("Test App"));
    }

    #[test]
    fn test_probe_response_serialization() {
        let resp = ProbeResponse {
            agent_id: "ccs-1".into(),
            state: "idle".into(),
            ip: Some(3),
            world_graph_size: 10,
            memory_trace_length: 3,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("ccs-1"));
    }

    #[test]
    fn test_http_listener_lifecycle() {
        let gw_state = Arc::new(HttpGatewayState {
            gateway: Arc::new(Mutex::new(GatewayState::new())),
        });
        let mut listener = HttpListener::new("0.0.0.0:8778", gw_state);
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        assert_eq!(listener.bound_address(), Some("0.0.0.0:8778".into()));
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
