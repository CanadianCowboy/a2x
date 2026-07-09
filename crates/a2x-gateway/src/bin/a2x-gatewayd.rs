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

    // Build config from env
    let mut cfg = GatewayConfig::default();
    if let Ok(sk) = env::var("A2X_API_KEY") {
        cfg.auth.mode = "api_key".into();
        cfg.auth.api_keys.push(ApiKeyEntry {
            key: sk,
            entity_id: "app-1".into(),
        });
    }

    // Chat backend config from env
    if let Ok(backend) = env::var("A2X_CHAT_BACKEND") {
        cfg.chat_backend.backend_type = backend;
    }
    if let Ok(model) = env::var("A2X_CHAT_MODEL") {
        cfg.chat_backend.model = model;
    }
    if let Ok(url) = env::var("A2X_CHAT_API_URL") {
        cfg.chat_backend.api_url = url;
    }
    if let Ok(key) = env::var("A2X_CHAT_API_KEY") {
        cfg.chat_backend.api_key = key;
    }
    if let Ok(ct) = env::var("A2X_CHAT_CONTEXT_TOKENS") {
        if let Ok(n) = ct.parse::<u32>() {
            cfg.chat_backend.max_context_tokens = n;
        }
    }

    // Initialize gateway from config and wrap state for HTTP
    let gw = Gateway::from_config(cfg).unwrap_or_else(|e| {
        eprintln!("a2x-gatewayd: failed to init gateway from config: {}", e);
        std::process::exit(1);
    });

    // Register built-in agents so the dashboard shows a live ecosystem.
    if let Err(e) = gw.register_builtin_agents() {
        eprintln!(
            "a2x-gatewayd: warning — failed to register built-in agents: {}",
            e
        );
    }

    // Bootstrap the WorldGraph with system concepts so the dashboard graph
    // has meaningful data on startup.
    {
        let state = gw.state_arc();
        let gw_state = state.lock().unwrap_or_else(|e| {
            eprintln!("a2x-gatewayd: failed to lock state for bootstrap: {}", e);
            std::process::exit(1);
        });
        if let Err(e) = gw_state.bootstrap_world_graph() {
            eprintln!("a2x-gatewayd: warning — WorldGraph bootstrap failed: {}", e);
        } else {
            println!("WorldGraph bootstrapped with system concepts");
        }
    }

    // Eagerly init the chat agent to validate backend connectivity.
    let gw_arc = gw.state_arc();
    {
        let mut state = gw_arc.lock().unwrap_or_else(|e| {
            eprintln!("a2x-gatewayd: failed to lock gateway state: {}", e);
            std::process::exit(1);
        });
        let _chat = state.get_chat_agent();
        match state.config.chat_backend.backend_type.as_str() {
            "ollama" => {
                println!(
                    "Chat agent: ollama / {} @ {}",
                    state.config.chat_backend.model, state.config.chat_backend.api_url
                );
            }
            "openai" => {
                println!("Chat agent: openai / {}", state.config.chat_backend.model);
            }
            _ => {
                println!(
                    "Chat agent: no backend configured (set A2X_CHAT_BACKEND=ollama to enable)"
                );
            }
        }
    }

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
