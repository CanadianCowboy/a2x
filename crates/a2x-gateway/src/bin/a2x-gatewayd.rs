use std::{env, sync::Arc};

use a2x_gateway::config::ApiKeyEntry;
use a2x_gateway::listeners::http::{HttpGatewayState, HttpListener};
use a2x_gateway::ProtocolListener;
use a2x_gateway::{Gateway, GatewayConfig};

fn main() {
    // Read address/port from env or default to 0.0.0.0:8778
    let addr = if let Ok(bind) = env::var("A2X_HTTP_ADDR") {
        bind
    } else {
        let port: u16 = env::var("A2X_HTTP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8778);
        format!("0.0.0.0:{}", port)
    };

    // Build config from env (API key optional)
    let mut cfg = GatewayConfig::default();
    if let Ok(sk) = env::var("A2X_API_KEY") {
        cfg.auth.mode = "api_key".into();
        cfg.auth.api_keys.push(ApiKeyEntry {
            key: sk,
            entity_id: "app-1".into(),
        });
    }

    // Initialize gateway from config and wrap state for HTTP
    let gw = Gateway::from_config(cfg).unwrap_or_else(|e| {
        eprintln!("a2x-gatewayd: failed to init gateway from config: {}", e);
        std::process::exit(1);
    });
    let gw_state = Arc::new(HttpGatewayState {
        gateway: gw.state_arc(),
    });

    // Start HTTP listener
    let mut http = HttpListener::new(addr.clone(), gw_state);
    if let Err(e) = http.start() {
        eprintln!(
            "a2x-gatewayd: failed to start HTTP listener on {}: {}",
            addr, e
        );
        std::process::exit(1);
    }

    println!("a2x-gatewayd is serving on http://{}", addr);
    println!(
        "Try: curl -sS -X POST -H \"Content-Type: application/json\" -d '{{\"program\":\"⟦Σ∞⟧⟬I:✕⟭\"}}' http://{}/a2x/execute",
        addr.replace("0.0.0.0", "127.0.0.1")
    );
    if std::env::var("A2X_API_KEY").is_ok() {
        println!(
            "Authenticated example: curl -sS 'http://{}/a2x/execute?api_key=$A2X_API_KEY' -H 'Content-Type: application/json' -d '{{\"program\":\"⟦Σ∞⟧⟬I:✕⟭\"}}'",
            addr.replace("0.0.0.0", "127.0.0.1")
        );
    }

    // Block until Ctrl+C, then shut down gracefully.
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        eprintln!("a2x-gatewayd: failed to set Ctrl+C handler: {}", e);
    }
    println!("Press Ctrl+C to stop.");
    let _ = rx.recv();

    println!("Shutting down...");
    if let Err(e) = http.stop() {
        eprintln!("a2x-gatewayd: error stopping HTTP listener: {}", e);
    }
    println!("a2x-gatewayd stopped.");
}
