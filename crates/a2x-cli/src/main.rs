// a2x-cli — CLI binary for interacting with the A2X system
//
// See plans/05-agents.md and PLAN.md §7 for the full design specification.
//
// Subcommands:
//   run     — Parse a Σ∞ program from a string or file and execute it
//   parse   — Parse a Σ∞ input and display each packet
//   agents  — List registered agents on the A2X bus
//   probe   — Inspect an agent's internal state

use std::fs;
use std::io::{self, Read, Write};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use a2x_agents::{CcsAgent, CliAgent, LlmAgent, Orchestrator};
use a2x_bus::{AgentFilter, AgentInfo, Bus};
use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_gateway::ProtocolListener;
use a2x_sigma::{parse_program, serialize_packet};

/// A2X — Agent-to-Anything: an AI-native programming language & runtime.
#[derive(Parser)]
#[command(
    name = "a2x",
    version,
    about = "A2X CLI — interact with the A2X agent ecosystem",
    long_about = "Execute Σ∞ programs, inspect agents, and manage the A2X bus."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Execute a Σ∞ program.
    ///
    /// Parses a Σ∞ program from a string argument or from stdin,
    /// dispatches it to an orchestrator agent, and prints the result.
    Run {
        /// Σ∞ program source to execute.
        ///
        /// Example: ⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭
        #[arg(short, long, group = "input")]
        program: Option<String>,

        /// Read the Σ∞ program from standard input.
        #[arg(short, long, group = "input")]
        stdin: bool,

        /// Read the Σ∞ program from a file.
        #[arg(short, long, group = "input")]
        file: Option<String>,
    },

    /// Parse and display a Σ∞ program.
    ///
    /// Parses a Σ∞ input string or file and displays each instruction
    /// packet in its human-readable text form.
    Parse {
        /// Σ∞ program source to parse.
        #[arg(short, long, group = "input")]
        program: Option<String>,

        /// Read the Σ∞ program from standard input.
        #[arg(short, long, group = "input")]
        stdin: bool,

        /// Read the Σ∞ program from a file.
        #[arg(short, long, group = "input")]
        file: Option<String>,

        /// Show verbose output including packet field details.
        #[arg(short, long)]
        verbose: bool,
    },

    /// List registered agents on the A2X bus.
    ///
    /// Creates a bus, registers built-in agents, and displays
    /// all agents with their capabilities and online status.
    Agents {
        /// Filter by agent type.
        #[arg(short, long)]
        type_filter: Option<String>,

        /// Filter by capability.
        #[arg(short, long)]
        capability: Option<String>,
    },

    /// Probe an agent's internal state.
    ///
    /// Creates an agent and displays its current state snapshot
    /// including WorldGraph size, MemoryTrace length, and IP.
    Probe {
        /// Agent ID to probe (e.g., "orch-1", "cli-1").
        agent_id: String,

        /// Agent type to create for probing.
        #[arg(short, long, default_value = "orchestrator")]
        agent_type: String,
    },

    /// Interactive Σ∞ REPL shell.
    ///
    /// Start an interactive read-eval-print loop for Σ∞ programs.
    /// Type programs directly and see results immediately.
    /// Special commands: :help, :quit, :agents, :parse <expr>
    Shell,

    /// Monitor the A2X bus in real-time.
    ///
    /// Displays live bus traffic — agents joining, programs executing, results flowing.
    Monitor {
        /// Gateway address to connect to (default: 127.0.0.1:8778).
        #[arg(short, long, default_value = "127.0.0.1:8778")]
        addr: String,
    },

    /// Launch the A2X web dashboard.
    ///
    /// Starts the A2X gateway daemon and serves the web dashboard.
    /// Set A2X_CHAT_BACKEND=ollama to enable the ChatAgent.
    Dashboard {
        /// Port to listen on (default: 8778).
        #[arg(short, long, default_value = "8778")]
        port: u16,

        /// Don't open the browser automatically.
        #[arg(long)]
        no_browser: bool,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // Initialize tracing subscriber for structured Σ∞ packet event logging.
    // Uses env-filter format: RUST_LOG=info,a2x_ccs=debug,a2x_ccs::vm=trace
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            program,
            stdin,
            file,
        } => cmd_run(program, stdin, file),
        Command::Parse {
            program,
            stdin,
            file,
            verbose,
        } => cmd_parse(program, stdin, file, verbose),
        Command::Agents {
            type_filter,
            capability,
        } => cmd_agents(type_filter, capability),
        Command::Probe {
            agent_id,
            agent_type,
        } => cmd_probe(&agent_id, &agent_type),
        Command::Shell => cmd_shell(),
        Command::Monitor { addr } => cmd_monitor(&addr),
        Command::Dashboard { port, no_browser } => cmd_dashboard(port, no_browser),
    }
}

// ---------------------------------------------------------------------------
// Subcommand: run
// ---------------------------------------------------------------------------

fn cmd_run(program: Option<String>, stdin: bool, file: Option<String>) -> Result<()> {
    let source = read_input(program, stdin, file).context("failed to read program input")?;

    // Parse the Σ∞ source into a SigmaProgram
    let mut sigma_program =
        parse_program(&source).map_err(|e| anyhow::anyhow!("failed to parse Σ∞ program: {}", e))?;
    sigma_program.compute_id();

    let instruction_count = sigma_program.len();
    println!(
        "→ Parsed {} instruction{} (program: {})",
        instruction_count,
        if instruction_count == 1 { "" } else { "s" },
        sigma_program.id,
    );

    // Create an orchestrator and dispatch the program
    let orchestrator = Orchestrator::new(AgentId::new("a2x-cli-orch"));
    let result = orchestrator
        .dispatch(sigma_program)
        .context("orchestrator failed to execute program")?;

    println!("✓ Execution complete");
    if !result.is_empty() {
        println!("  Result: {} instruction(s)", result.len());
        for (i, pkt) in result.instructions.iter().enumerate() {
            println!("    [{}] {}", i, serialize_packet(pkt));
        }
    } else {
        println!("  Result: (empty program)");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: parse
// ---------------------------------------------------------------------------

fn cmd_parse(
    program: Option<String>,
    stdin: bool,
    file: Option<String>,
    verbose: bool,
) -> Result<()> {
    let source = read_input(program, stdin, file).context("failed to read parse input")?;

    // Parse and compute ID
    let mut sigma_program =
        parse_program(&source).map_err(|e| anyhow::anyhow!("failed to parse Σ∞ program: {}", e))?;
    sigma_program.compute_id();

    let count = sigma_program.len();
    println!("Program ID: {}", sigma_program.id);
    println!("Instructions: {}", count);
    println!("Labels: {}", sigma_program.labels.len());
    println!("Sub-programs: {}", sigma_program.sub_programs.len());
    println!();

    if count == 0 {
        println!("(empty program)");
        return Ok(());
    }

    for (i, pkt) in sigma_program.instructions.iter().enumerate() {
        println!("── Instruction {} ──", i);
        println!("  {}", serialize_packet(pkt));

        if verbose {
            // Show field breakdown
            if !pkt.intent.is_empty() {
                print!("  I (intent):  ");
                for op in &pkt.intent.operators {
                    print!("{:?} ", op);
                }
                println!();
            }
            if !pkt.context.is_empty() {
                print!("  C (context): ");
                for op in &pkt.context.operators {
                    print!("{:?} ", op);
                }
                for label in &pkt.context.labels {
                    print!("⟨{}⟩ ", label);
                }
                println!();
            }
            if !pkt.plan.is_empty() {
                print!("  P (plan):    ");
                for op in &pkt.plan.operators {
                    print!("{:?} ", op);
                }
                println!();
            }
            if !pkt.data.is_empty() {
                print!("  D (data):    ");
                for op in &pkt.data.operators {
                    print!("{:?} ", op);
                }
                if !pkt.data.payload.is_empty() {
                    print!(" [{} bytes]", pkt.data.payload.len());
                }
                println!();
            }
        }
        println!();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: agents
// ---------------------------------------------------------------------------

fn cmd_agents(type_filter: Option<String>, capability: Option<String>) -> Result<()> {
    let mut bus = Bus::new();

    // Register built-in agents on the bus
    register_builtin_agents(&mut bus);

    // Determine filter
    let filter = if let Some(cap) = capability {
        let c = parse_capability(&cap)?;
        AgentFilter::ByCapability(c)
    } else if let Some(t) = type_filter {
        let at = parse_agent_type(&t)?;
        AgentFilter::ByType(at)
    } else {
        AgentFilter::All
    };

    let agents = bus.discover(&filter);

    if agents.is_empty() {
        println!("No agents found.");
        return Ok(());
    }

    println!("{:>6}  {:20} {:8}  Capabilities", "ID", "Type", "Online");
    println!("{}", "─".repeat(80));

    for info in &agents {
        let status_icon = if info.online { "●" } else { "○" };
        let caps: Vec<String> = info
            .capabilities
            .iter()
            .map(|c: &Capability| c.to_string())
            .collect();

        println!(
            "{:>6}  {:20} {:8}  {}",
            info.id.as_str(),
            format!("{:?}", info.agent_type),
            status_icon,
            caps.join(", "),
        );
    }

    println!();
    println!("Total: {} agent(s)", agents.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: probe
// ---------------------------------------------------------------------------

fn cmd_probe(agent_id: &str, agent_type: &str) -> Result<()> {
    let id = AgentId::new(agent_id);

    // Create the appropriate agent type and show its state
    let snapshot = match agent_type.to_lowercase().as_str() {
        "orchestrator" => {
            let agent = Orchestrator::new(id);
            agent.state_summary()
        }
        "cli" => {
            let agent = CliAgent::new(id);
            agent.state_summary()
        }
        "llm" => {
            let agent = LlmAgent::new_stub(id, "probe-model");
            agent.state_summary()
        }
        "ccs" => {
            let agent = CcsAgent::new(id);
            agent.state_summary()
        }
        other => anyhow::bail!(
            "unknown agent type '{}'. Valid types: orchestrator, cli, llm, ccs",
            other
        ),
    };

    match snapshot {
        Some(s) => {
            println!("Agent:       {}", s.agent_id);
            println!("State:       {}", s.state);
            println!(
                "Program:     {}",
                s.current_program
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "(none)".into())
            );
            println!(
                "IP:          {}",
                s.ip.map(|v| v.to_string())
                    .unwrap_or_else(|| "(none)".into())
            );
            println!("WorldGraph:  {} node(s)", s.world_graph_size);
            println!("MemoryTrace: {} entrie(s)", s.memory_trace_length);
            println!("Uptime:      {:.1}s", s.uptime.as_secs_f32());
        }
        None => {
            println!("No state available for agent '{}'.", agent_id);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read program input from one of three sources: direct argument, stdin, or file.
fn read_input(direct: Option<String>, stdin_flag: bool, file: Option<String>) -> Result<String> {
    if let Some(s) = direct {
        return Ok(s);
    }

    if stdin_flag {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        return Ok(buf);
    }

    if let Some(path) = file {
        return fs::read_to_string(&path)
            .with_context(|| format!("failed to read file '{}'", path));
    }

    // No input source specified
    anyhow::bail!("no input specified. Use --program, --stdin, or --file.")
}

/// Parse a string into a Capability.
fn parse_capability(s: &str) -> Result<Capability> {
    match s.to_lowercase().as_str() {
        "execute" => Ok(Capability::Execute),
        "filesystem" | "fs" => Ok(Capability::FileSystem),
        "network" | "net" => Ok(Capability::Network),
        "shell" | "exec" => Ok(Capability::Shell),
        "probe" => Ok(Capability::Probe),
        other => Ok(Capability::Custom(other.into())),
    }
}

/// Parse a string into an AgentType.
fn parse_agent_type(s: &str) -> Result<AgentType> {
    match s.to_lowercase().as_str() {
        "orchestrator" | "orch" => Ok(AgentType::Orchestrator),
        "cli" => Ok(AgentType::Cli),
        "llm" => Ok(AgentType::Llm),
        "ccs" => Ok(AgentType::Ccs),
        "omega" => Ok(AgentType::Omega),
        "entity" => Ok(AgentType::Entity),
        other => anyhow::bail!(
            "unknown agent type '{}'. Valid types: orchestrator, cli, llm, ccs, omega, entity",
            other
        ),
    }
}

// ---------------------------------------------------------------------------
// Subcommand: shell
// ---------------------------------------------------------------------------

fn cmd_shell() -> Result<()> {
    let cyan = "\x1b[36m";
    let green = "\x1b[32m";
    let yellow = "\x1b[33m";
    let red = "\x1b[31m";
    let reset = "\x1b[0m";

    println!(
        "{}═══════════════════════════════════════════{}",
        cyan, reset
    );
    println!("{}  A2X Shell — Interactive Σ∞ REPL{}", cyan, reset);
    println!("{}  Type Σ∞ programs or :help for commands{}", cyan, reset);
    println!(
        "{}═══════════════════════════════════════════{}\n",
        cyan, reset
    );

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("{green}Σ∞>{reset} ");
        let _ = io::stdout().flush();
        input.clear();
        match stdin.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {
                let line = input.trim();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with(':') {
                    match line {
                        ":quit" | ":q" => break,
                        ":help" | ":h" => {
                            println!("  :quit, :q       Exit the REPL");
                            println!("  :help, :h       Show this help");
                            println!("  :agents         List built-in agents");
                            println!("  :parse <expr>   Parse and display a Σ∞ expression");
                            println!("  :monitor        Show bus state");
                            println!("  <Σ∞ program>    Parse and execute a program");
                        }
                        ":agents" => {
                            let mut bus = Bus::new();
                            register_builtin_agents(&mut bus);
                            let agents = bus.discover(&AgentFilter::All);
                            for info in &agents {
                                let caps: Vec<String> =
                                    info.capabilities.iter().map(|c| c.to_string()).collect();
                                println!(
                                    "  {}○ {}{} {} {:?}{} {}",
                                    green,
                                    info.id.as_str(),
                                    reset,
                                    yellow,
                                    info.agent_type,
                                    reset,
                                    caps.join(", ")
                                );
                            }
                            println!("  {}— {} agent(s){}", cyan, agents.len(), reset);
                        }
                        ":monitor" => {
                            let mut bus = Bus::new();
                            register_builtin_agents(&mut bus);
                            println!(
                                "  {}Bus state:{} {} agent(s) registered",
                                cyan,
                                reset,
                                bus.agent_count()
                            );
                            let all = bus.discover(&AgentFilter::All);
                            for info in &all {
                                println!(
                                    "    {} → {:?} [{}]",
                                    info.id.as_str(),
                                    info.agent_type,
                                    if info.online { "online" } else { "offline" }
                                );
                            }
                        }
                        _ if line.starts_with(":parse ") => {
                            let expr = &line[7..];
                            match parse_program(expr) {
                                Ok(mut prog) => {
                                    prog.compute_id();
                                    println!(
                                        "  {}✓ Parsed{} — {} instruction(s), ID: {}",
                                        green,
                                        reset,
                                        prog.len(),
                                        prog.id
                                    );
                                    for (i, pkt) in prog.instructions.iter().enumerate() {
                                        println!("    [{}] {}", i, serialize_packet(pkt));
                                    }
                                }
                                Err(e) => println!("  {}✗ Parse error:{} {}", red, reset, e),
                            }
                        }
                        _ => println!("  {}Unknown command:{} {}", yellow, reset, line),
                    }
                } else {
                    match parse_program(line) {
                        Ok(mut prog) => {
                            prog.compute_id();
                            let orchestrator = Orchestrator::new(AgentId::new("shell-orch"));
                            match orchestrator.dispatch(prog) {
                                Ok(result) => {
                                    let out = if result.is_empty() {
                                        "∅ (empty)".to_string()
                                    } else {
                                        result
                                            .instructions
                                            .iter()
                                            .map(serialize_packet)
                                            .collect::<Vec<_>>()
                                            .join("\n    ")
                                    };
                                    println!("  {}✓ {}", green, out);
                                }
                                Err(e) => println!("  {}✗ Dispatch:{} {}", red, reset, e),
                            }
                        }
                        Err(e) => println!("  {}✗ Parse:{} {}", red, reset, e),
                    }
                }
            }
            Err(e) => {
                eprintln!("  {}Read error:{} {}", red, reset, e);
                break;
            }
        }
    }
    println!("\n{}Goodbye.{}", cyan, reset);
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: monitor
// ---------------------------------------------------------------------------

fn cmd_monitor(_addr: &str) -> Result<()> {
    let cyan = "\x1b[36m";
    let reset = "\x1b[0m";

    println!("{}══ A2X Bus Monitor ══{}", cyan, reset);
    println!();

    let mut bus = Bus::new();
    register_builtin_agents(&mut bus);

    let all = bus.discover(&AgentFilter::All);
    println!("Connected agents: {}", all.len());
    println!("{:>6}  {:20} {:8}  Capabilities", "ID", "Type", "Status");
    println!("{}", "─".repeat(70));
    for info in &all {
        let caps: Vec<String> = info.capabilities.iter().map(|c| c.to_string()).collect();
        println!(
            "{:>6}  {:20} {:8}  {}",
            info.id.as_str(),
            format!("{:?}", info.agent_type),
            if info.online {
                "● online"
            } else {
                "○ offline"
            },
            caps.join(", "),
        );
    }

    // Execute a demo program to show bus activity
    println!("\nExecuting demo program...");
    let orch = Orchestrator::new(AgentId::new("monitor-orch"));
    let mut prog = a2x_sigma::program::SigmaProgram::new();
    prog.push(a2x_sigma::SigmaPacket::default());
    prog.compute_id();

    let prog_id = prog.id;
    match orch.dispatch(prog) {
        Ok(result) => {
            println!(
                "  Dispatch: {} → {} result instruction(s)",
                prog_id,
                result.len()
            );
        }
        Err(e) => {
            println!("  Dispatch error: {}", e);
        }
    }

    println!("\nTip: Connect to a running gateway for live traffic:");
    println!("  a2x dashboard    # start the gateway + web UI");
    println!("  Then visit http://127.0.0.1:8778 in your browser");
    Ok(())
}

// ---------------------------------------------------------------------------
// Subcommand: dashboard
// ---------------------------------------------------------------------------

fn cmd_dashboard(port: u16, no_browser: bool) -> Result<()> {
    let cyan = "\x1b[36m";
    let green = "\x1b[32m";
    let yellow = "\x1b[33m";
    let reset = "\x1b[0m";

    println!("{}══ A2X Dashboard ══{}", cyan, reset);
    println!();

    let addr = format!("0.0.0.0:{}", port);

    // Create gateway with default config
    // Create gateway with config from env vars (same as a2x-gatewayd)
    let mut cfg = a2x_gateway::GatewayConfig::default();
    if let Ok(sk) = std::env::var("A2X_API_KEY") {
        cfg.auth.mode = "api_key".into();
        cfg.auth.api_keys.push(a2x_gateway::config::ApiKeyEntry {
            key: sk,
            entity_id: "app-1".into(),
        });
    }
    if let Ok(backend) = std::env::var("A2X_CHAT_BACKEND") {
        cfg.chat_backend.backend_type = backend;
    }
    if let Ok(model) = std::env::var("A2X_CHAT_MODEL") {
        cfg.chat_backend.model = model;
    }
    if let Ok(url) = std::env::var("A2X_CHAT_API_URL") {
        cfg.chat_backend.api_url = url;
    }
    if let Ok(key) = std::env::var("A2X_CHAT_API_KEY") {
        cfg.chat_backend.api_key = key;
    }
    if let Ok(ct) = std::env::var("A2X_CHAT_CONTEXT_TOKENS") {
        if let Ok(n) = ct.parse::<u32>() {
            cfg.chat_backend.max_context_tokens = n;
        }
    }

    let gw = a2x_gateway::Gateway::from_config(cfg)
        .map_err(|e| anyhow::anyhow!("Failed to create gateway: {}", e))?;

    // Register built-in agents
    if let Err(e) = gw.register_builtin_agents() {
        println!(
            "  {}Warning:{} failed to register agents: {}",
            yellow, reset, e
        );
    }

    let gw_state = std::sync::Arc::new(a2x_gateway::listeners::http::HttpGatewayState {
        gateway: gw.state_arc(),
    });

    // Start HTTP listener
    let mut http = a2x_gateway::listeners::http::HttpListener::new(addr.clone(), gw_state);
    if let Err(e) = http.start() {
        anyhow::bail!("Failed to start HTTP listener on {}: {}", addr, e);
    }

    let local_addr = addr.replace("0.0.0.0", "127.0.0.1");
    println!(
        "  {}✓{} Dashboard running at {green}http://{}{}",
        green, reset, local_addr, reset
    );
    println!(
        "  Chat backend: {yellow}{}{} (set A2X_CHAT_BACKEND=ollama to enable)",
        std::env::var("A2X_CHAT_BACKEND").unwrap_or_else(|_| "none".into()),
        reset
    );
    println!("  Press Ctrl+C to stop.\n");

    // Open browser
    if !no_browser {
        let url = format!("http://{}", local_addr);
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", &url])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&url).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        }
    }

    // Block until Ctrl+C
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        println!("  {}Warning:{} Ctrl+C handler: {}", yellow, reset, e);
    }
    let _ = rx.recv();

    println!("\nShutting down...");
    if let Err(e) = http.stop() {
        println!("  {}Warning:{} error stopping: {}", yellow, reset, e);
    }
    println!("{}Dashboard stopped.{}", cyan, reset);
    Ok(())
}

/// Register built-in agents on the bus for the `agents` subcommand.
fn register_builtin_agents(bus: &mut Bus) {
    let agents: Vec<AgentInfo> = vec![
        AgentInfo::new(
            AgentId::new("orch-1"),
            AgentType::Orchestrator,
            vec![Capability::Execute, Capability::Custom("schedule".into())],
        ),
        AgentInfo::new(
            AgentId::new("cli-1"),
            AgentType::Cli,
            vec![
                Capability::Execute,
                Capability::FileSystem,
                Capability::Network,
                Capability::Shell,
            ],
        ),
        AgentInfo::new(
            AgentId::new("cli-2"),
            AgentType::Cli,
            vec![Capability::Execute, Capability::FileSystem],
        ),
        AgentInfo::new(
            AgentId::new("llm-1"),
            AgentType::Llm,
            vec![Capability::Execute, Capability::Custom("plan".into())],
        ),
        AgentInfo::new(
            AgentId::new("ccs-1"),
            AgentType::Ccs,
            vec![
                Capability::Execute,
                Capability::Custom("plan".into()),
                Capability::Custom("schedule".into()),
            ],
        ),
    ];

    for info in agents {
        let _ = bus.register_agent(info);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capability_known() {
        assert_eq!(parse_capability("execute").unwrap(), Capability::Execute);
        assert_eq!(parse_capability("fs").unwrap(), Capability::FileSystem);
        assert_eq!(parse_capability("net").unwrap(), Capability::Network);
        assert_eq!(parse_capability("shell").unwrap(), Capability::Shell);
        assert_eq!(parse_capability("probe").unwrap(), Capability::Probe);
    }

    #[test]
    fn test_parse_capability_custom() {
        let cap = parse_capability("custom-thing").unwrap();
        assert_eq!(cap.to_string(), "custom:custom-thing");
    }

    #[test]
    fn test_parse_agent_type_known() {
        assert_eq!(
            parse_agent_type("orchestrator").unwrap(),
            AgentType::Orchestrator
        );
        assert_eq!(parse_agent_type("orch").unwrap(), AgentType::Orchestrator);
        assert_eq!(parse_agent_type("cli").unwrap(), AgentType::Cli);
        assert_eq!(parse_agent_type("llm").unwrap(), AgentType::Llm);
        assert_eq!(parse_agent_type("ccs").unwrap(), AgentType::Ccs);
        assert_eq!(parse_agent_type("omega").unwrap(), AgentType::Omega);
    }

    #[test]
    fn test_parse_agent_type_unknown() {
        assert!(parse_agent_type("foobar").is_err());
    }

    #[test]
    fn test_read_input_from_direct() {
        let result = read_input(Some("hello".into()), false, None).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_read_input_no_source() {
        let result = read_input(None, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_probe_orchestrator() {
        let result = cmd_probe("test-orch", "orchestrator");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_probe_cli() {
        let result = cmd_probe("test-cli", "cli");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_probe_llm() {
        let result = cmd_probe("test-llm", "llm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_probe_ccs() {
        let result = cmd_probe("test-ccs", "ccs");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_probe_unknown_type() {
        let result = cmd_probe("test-x", "unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_agents_all() {
        let result = cmd_agents(None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_agents_filter_by_type() {
        let result = cmd_agents(Some("cli".into()), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_agents_filter_by_capability() {
        let result = cmd_agents(None, Some("execute".into()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_parse_valid() {
        let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
        let result = cmd_parse(Some(input.into()), false, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_parse_verbose() {
        let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
        let result = cmd_parse(Some(input.into()), false, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_parse_empty_program() {
        let result = cmd_parse(Some("".into()), false, None, false);
        // Empty string produces empty program, which is valid
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_parse_invalid_input() {
        let result = cmd_parse(Some("⟦not valid sigma⟧".into()), false, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_run_valid() {
        // A simple HALT program — Cancel intent + Cancel plan = HALT opcode
        let input = "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭";
        let result = cmd_run(Some(input.into()), false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_run_empty() {
        let result = cmd_run(Some("".into()), false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_builtin_agents() {
        let mut bus = Bus::new();
        register_builtin_agents(&mut bus);
        assert_eq!(bus.agent_count(), 5);
    }

    /// Integration test: parse → run roundtrip
    #[test]
    fn test_parse_and_run_roundtrip() {
        // A simple NOP program
        let input = "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭";
        let result = cmd_run(Some(input.into()), false, None);
        assert!(result.is_ok());
    }
}
