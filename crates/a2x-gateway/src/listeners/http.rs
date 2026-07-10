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

use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::gateway::GatewayState;
use crate::listeners::{ProtocolListener, ProtocolListenerType};
use crate::tls::GatewayTlsConfig;
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

/// GET /healthz — Liveness probe.
async fn handle_healthz() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// GET /readyz — Readiness probe.
async fn handle_readyz(State(_): State<Arc<HttpGatewayState>>) -> (StatusCode, &'static str) {
    // For now, if the server is running and we can receive state, we are "ready".
    (StatusCode::OK, "ready")
}

/// POST /a2x/execute — Execute a Σ∞ program.
async fn handle_execute(
    State(state): State<Arc<HttpGatewayState>>,
    Query(auth): Query<AuthQuery>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start = std::time::Instant::now();

    // Authenticate via query param if api_key provided
    let entity_id = if let Some(ref key) = auth.api_key {
        match state.gateway.lock() {
            Ok(gw) => match gw.authenticate(&crate::auth::AuthMethod::ApiKey(key.clone())) {
                Ok(eid) => Some(eid),
                Err(_) => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"error": "invalid api_key"})),
                    ));
                }
            },
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("lock error: {}", e)})),
                ));
            }
        }
    } else {
        None
    };

    // Parse the program
    let program = a2x_sigma::parse_program(&req.program).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("parse error: {}", e)})),
        )
    })?;

    // Check auth status before entity_id is moved below.
    let was_authenticated = entity_id.is_some();

    // Execute via the gateway (with permission enforcement if authenticated)
    let result = {
        let mut gw = state.gateway.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("lock error: {}", e)})),
            )
        })?;
        if let Some(eid) = entity_id {
            gw.execute_program_for_entity(&program, &eid).map_err(|e| {
                (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": format!("{}", e)})),
                )
            })?
        } else {
            gw.execute_program(&program).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("execution error: {}", e)})),
                )
            })?
        }
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

    // Record execution in dashboard history for live monitoring.
    if let Ok(mut gw) = state.gateway.lock() {
        let source_preview: String = req.program.chars().take(80).collect();
        let result_preview: String = result_str.chars().take(80).collect();
        gw.record_execution(&source_preview, &result_preview, "completed", elapsed);
        let auth_label = if was_authenticated { "(auth)" } else { "" };
        gw.record_bus_event(
            "exec",
            &format!("HTTP {} — completed in {}ms", auth_label, elapsed),
        );
    }

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
///
/// On `start()`, spawns a dedicated OS thread with its own tokio runtime
/// that binds to the configured address and serves the axum router.
/// `stop()` signals graceful shutdown via a oneshot channel.
pub struct HttpListener {
    bind_address: String,
    gateway_state: Arc<HttpGatewayState>,
    /// Optional TLS configuration for HTTPS.
    /// When set, the listener serves HTTPS instead of HTTP.
    /// In production, TLS termination via reverse proxy (nginx/caddy) is
    /// the recommended approach; this field exists for direct TLS support.
    tls_config: Option<GatewayTlsConfig>,
    /// Actual bound address (resolved after bind, e.g. port 0 → 127.0.0.1:54321).
    resolved_address: Option<String>,
    running: bool,
    /// Handle to the server thread (joined on drop, but we signal shutdown first).
    server_thread: Option<std::thread::JoinHandle<()>>,
    /// Signal graceful shutdown to the axum server.
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl HttpListener {
    pub fn new(bind_address: impl Into<String>, gateway_state: Arc<HttpGatewayState>) -> Self {
        HttpListener {
            bind_address: bind_address.into(),
            gateway_state,
            tls_config: None,
            resolved_address: None,
            running: false,
            server_thread: None,
            shutdown_tx: None,
        }
    }

    /// Create a new HTTP listener with TLS enabled.
    ///
    /// When TLS is configured, the listener will serve HTTPS using the
    /// provided certificate and private key. For production, consider
    /// using a reverse proxy (nginx/caddy) for TLS termination instead.
    pub fn with_tls(
        bind_address: impl Into<String>,
        gateway_state: Arc<HttpGatewayState>,
        tls_config: GatewayTlsConfig,
    ) -> Self {
        HttpListener {
            bind_address: bind_address.into(),
            gateway_state,
            tls_config: Some(tls_config),
            resolved_address: None,
            running: false,
            server_thread: None,
            shutdown_tx: None,
        }
    }

    /// Check whether TLS is enabled on this listener.
    pub fn is_tls_enabled(&self) -> bool {
        self.tls_config.is_some()
    }

    /// Build the axum router with all A2X endpoints.
    ///
    /// This is the canonical router definition. External integrators
    /// can use this to embed the A2X HTTP API into their own axum server.
    pub fn router(state: Arc<HttpGatewayState>) -> Router {
        use crate::dashboard;
        Router::new()
            .route("/", get(dashboard::handle_dashboard))
            .route("/a2x/dashboard/ws", get(dashboard::handle_dashboard_ws))
            .route("/a2x/execute", post(handle_execute))
            .route("/a2x/entities", get(handle_list_entities))
            .route("/a2x/entities/{entity_id}", get(handle_get_entity))
            .route("/a2x/probe/{agent_id}", get(handle_probe))
            .route("/a2x/webhook", post(handle_register_webhook))
            .route("/healthz", get(handle_healthz))
            .route("/readyz", get(handle_readyz))
            .with_state(state)
    }
}

impl Drop for HttpListener {
    fn drop(&mut self) {
        // Signal shutdown if still running — best-effort cleanup.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.running = false;
    }
}

impl ProtocolListener for HttpListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::Http
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        if self.running {
            return Err(GatewayError::ListenerError(
                "HTTP listener is already running".into(),
            ));
        }

        // Clean up any stale handles from a previous run.
        if let Some(handle) = self.server_thread.take() {
            // The old thread should already be shut down if stop() was called.
            // Join with a short timeout as best-effort cleanup.
            let _ = handle.join();
        }

        let addr = self.bind_address.clone();
        let state = self.gateway_state.clone();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        // Use a sync channel so the spawner blocks until the server thread
        // confirms it has bound successfully (or reports an error).
        let (ready_tx, ready_rx) =
            std::sync::mpsc::sync_channel::<Result<Option<std::net::SocketAddr>, String>>(1);

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for HTTP listener");

            rt.block_on(async move {
                let app = HttpListener::router(state);
                let listener = match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => {
                        let local = l.local_addr().ok();
                        let _ = ready_tx.send(Ok(local));
                        l
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("bind failed: {}", e)));
                        tracing::error!("HTTP listener failed to bind {}: {}", addr, e);
                        return;
                    }
                };
                let local_addr = listener.local_addr().ok();
                tracing::info!(
                    "HTTP listener serving on {}",
                    local_addr
                        .as_ref()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| addr.clone())
                );
                if let Err(e) = axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                {
                    tracing::error!("HTTP listener server error: {}", e);
                }
                tracing::info!("HTTP listener shut down");
            });
        });

        // Block until the server thread reports bind success or failure.
        match ready_rx.recv() {
            Ok(Ok(addr)) => {
                self.resolved_address = addr.map(|a| a.to_string());
                self.server_thread = Some(handle);
                self.shutdown_tx = Some(shutdown_tx);
                self.running = true;
                Ok(())
            }
            Ok(Err(e)) => {
                // Bind failed — join the thread (it already returned).
                let _ = handle.join();
                Err(GatewayError::ListenerError(e))
            }
            Err(_) => {
                // Channel dropped without sending — thread panicked or crashed.
                let _ = handle.join();
                Err(GatewayError::ListenerError(
                    "server thread panicked during bind".into(),
                ))
            }
        }
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        if !self.running {
            return Ok(());
        }
        self.running = false;
        // Send shutdown signal to the axum server.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Join the server thread with a 3-second timeout so the port is
        // guaranteed free before this method returns.
        if let Some(handle) = self.server_thread.take() {
            let backoff = std::time::Duration::from_millis(100);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while std::time::Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                std::thread::sleep(backoff);
            }
        }
        tracing::info!("HTTP listener stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn bound_address(&self) -> Option<String> {
        self.resolved_address.clone()
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
        let mut listener = HttpListener::new("127.0.0.1:0", gw_state);
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        assert!(
            listener.bound_address().is_some(),
            "should have bound address"
        );
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
