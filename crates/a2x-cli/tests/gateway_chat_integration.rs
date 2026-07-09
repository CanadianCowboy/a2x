// gateway_chat_integration.rs
// End-to-end integration test: Gateway start -> HTTP execute -> verify response.
//
// Tests the full pipeline: config -> gateway -> HTTP listener -> program execution -> result.
// Run: cargo test -p a2x-cli --test gateway_chat_integration

use std::sync::Arc;

use a2x_core::graph::WorldGraph;
use a2x_gateway::listeners::http::{HttpGatewayState, HttpListener};
use a2x_gateway::{Gateway, GatewayConfig, ProtocolListener};

/// Test that the WorldGraph is bootstrapped with nodes on startup.
#[test]
fn test_world_graph_bootstrapped() {
    let cfg = GatewayConfig::default();
    let gw = Gateway::from_config(cfg).expect("create gateway");

    // Bootstrap should populate the WorldGraph
    let state = gw.state.lock().unwrap();
    state.bootstrap_world_graph().expect("bootstrap");

    let vm = state.chat_ccs_vm.lock().unwrap();
    let nodes = vm.world_graph.node_count();
    let edges = vm.world_graph.edge_count();

    assert!(
        nodes >= 12,
        "bootstrap should create at least 12 concept nodes, got {}",
        nodes
    );
    assert!(
        edges >= 10,
        "bootstrap should create at least 10 relation edges, got {}",
        edges
    );

    // Verify key labels exist
    for label in &["sys", "orch", "cli", "llm", "ccs", "bus", "gw"] {
        let found = vm.world_graph.lookup_label(label).unwrap();
        assert!(
            found.is_some(),
            "label '{}' should exist after bootstrap",
            label
        );
    }
}

/// Start the gateway on a random port, execute a Sigma program via the HTTP
/// execute endpoint, and verify the response.
#[test]
fn test_gateway_http_execute_end_to_end() {
    // Create gateway with default config
    let cfg = GatewayConfig::default();
    let gw = Gateway::from_config(cfg).expect("create gateway");

    // Register built-in agents
    gw.register_builtin_agents()
        .expect("register builtin agents");

    // Bootstrap WorldGraph
    {
        let state = gw.state.lock().unwrap();
        state
            .bootstrap_world_graph()
            .expect("bootstrap world graph");
    }

    // Start HTTP listener on a random port (port 0 = OS picks)
    let http_state = Arc::new(HttpGatewayState {
        gateway: gw.state_arc(),
    });
    let mut http = HttpListener::new("127.0.0.1:0", http_state);
    http.start().expect("start HTTP listener");

    // Get the actual bound port
    let addr = http.bound_address().expect("should have bound address");

    // Build the execute URL (bound to 127.0.0.1 so no replace needed)
    let url = format!("http://{}/a2x/execute", addr);

    // Execute a simple HALT program via HTTP
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let response = runtime
        .block_on(async {
            reqwest::Client::new()
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({
                    "program": "\u{27e6}\u{03a3}\u{221e}\u{27e7}\u{27ec}I:\u{2715} \u{2237} P:\u{2715}\u{27ed}"
                }))
                .send()
                .await
        })
        .expect("HTTP request should succeed");

    assert!(
        response.status().is_success(),
        "HTTP status should be 200 OK, got {}",
        response.status()
    );

    let body: serde_json::Value = runtime
        .block_on(async { response.json().await })
        .expect("parse JSON response");

    assert!(
        body.get("result").is_some(),
        "response should have 'result' field"
    );

    // Stop listener
    http.stop().expect("stop HTTP listener");
}
