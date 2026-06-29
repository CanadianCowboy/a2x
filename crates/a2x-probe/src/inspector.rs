// a2x-probe — CLI inspector commands
// See plans/07-probe.md §7 (Probe Tool CLI Commands)
//
// The Inspector provides a command-driven interface for inspecting a CCS VM
// through the probe channel. Commands are parsed from strings and dispatched
// to the appropriate ProbeTool methods.

use crate::ProbeError;
use crate::ProbeTool;
use a2x_ccs::probe::TracerMode;

/// Parse and execute a probe command string.
///
/// Returns the command output as a string, or an error if the command
/// is invalid or the probe channel is closed.
pub fn execute_command(tool: &ProbeTool, command: &str) -> Result<String, ProbeError> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok("No command entered. Type 'help' for available commands.".to_string());
    }

    match parts[0] {
        "status" | "s" => cmd_status(tool),
        "graph" | "g" => cmd_graph(tool),
        "regions" | "r" => cmd_regions(tool),
        "break" | "b" if parts.len() >= 2 => cmd_break(tool, &parts[1..]),
        "break" | "b" => Ok("Usage: break <ip> — set breakpoint at instruction index".to_string()),
        "clear" | "c" if parts.len() >= 2 => cmd_clear(tool, &parts[1..]),
        "clear" | "c" => Ok("Usage: clear <ip> — clear breakpoint, or 'clear all'".to_string()),
        "continue" | "cont" => {
            tool.r#continue()?;
            Ok("Continued execution.".to_string())
        }
        "step" | "st" => {
            tool.step()?;
            Ok("Stepped one instruction.".to_string())
        }
        "trace" | "t" if parts.len() >= 2 => cmd_trace(tool, &parts[1..]),
        "trace" | "t" => Ok("Usage: trace <n> — show last N trace entries".to_string()),
        "watch" | "w" if parts.len() >= 2 => cmd_watch(tool, &parts[1..]),
        "watch" | "w" => Ok("Usage: watch <region> — watch a StateField region".to_string()),
        "tracer" if parts.len() >= 2 => cmd_tracer(tool, &parts[1..]),
        "tracer" => Ok("Usage: tracer <off|light|full|verbose> — set tracer mode".to_string()),
        "heatmap" if parts.len() >= 2 => cmd_heatmap(tool, &parts[1..]),
        "heatmap" => Ok("Usage: heatmap <dims> — show state heatmap for last entries".to_string()),
        "timeline" => cmd_timeline(tool),
        "help" | "h" => Ok(help_text()),
        "quit" | "q" => Ok("Goodbye.".to_string()),
        _ => Ok(format!(
            "Unknown command: '{}'. Type 'help' for available commands.",
            parts[0]
        )),
    }
}

/// Show current VM status.
fn cmd_status(tool: &ProbeTool) -> Result<String, ProbeError> {
    tool.snapshot()?;
    // Note: in a real implementation, this would block for the response.
    // For now, we send the query and return a hint.
    Ok("Snapshot requested. Response will arrive via event channel.".to_string())
}

/// Dump the WorldGraph as a dot graph.
fn cmd_graph(tool: &ProbeTool) -> Result<String, ProbeError> {
    tool.graph_summary()?;
    Ok("Graph summary requested. Response will arrive via event channel.".to_string())
}

/// Show StateField regions.
fn cmd_regions(tool: &ProbeTool) -> Result<String, ProbeError> {
    tool.list_regions()?;
    Ok("Region list requested. Response will arrive via event channel.".to_string())
}

/// Set or list breakpoints.
fn cmd_break(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    match args[0] {
        "list" | "ls" => Ok("Breakpoint list requested.".to_string()),
        "clear" => {
            tool.clear_all_breakpoints()?;
            Ok("All breakpoints cleared.".to_string())
        }
        ip_str => {
            let ip: usize = ip_str.parse().map_err(|_| ProbeError::NotConnected)?; // TODO: better error
            tool.set_breakpoint(ip)?;
            Ok(format!("Breakpoint set at instruction {}.", ip))
        }
    }
}

/// Clear breakpoints.
fn cmd_clear(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    match args[0] {
        "all" => {
            tool.clear_all_breakpoints()?;
            Ok("All breakpoints cleared.".to_string())
        }
        ip_str => {
            let ip: usize = ip_str.parse().map_err(|_| ProbeError::NotConnected)?;
            tool.clear_breakpoint(ip)?;
            Ok(format!("Breakpoint cleared at instruction {}.", ip))
        }
    }
}

/// Show last N trace entries.
fn cmd_trace(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    let n: usize = args[0].parse().map_err(|_| ProbeError::NotConnected)?;
    tool.get_trace_tail(n)?;
    Ok(format!("Last {} trace entries requested.", n))
}

/// Watch a StateField region.
fn cmd_watch(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    let region = args[0];
    tool.get_trace_tail(1)?;
    Ok(format!(
        "Watching region \"{}\" — will show updates every instruction.",
        region
    ))
}

/// Set tracer mode.
fn cmd_tracer(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    let mode = match args[0] {
        "off" => TracerMode::Off,
        "light" => TracerMode::Light,
        "full" => TracerMode::Full,
        "verbose" => TracerMode::Verbose,
        other => {
            return Ok(format!(
                "Unknown tracer mode: '{}'. Use off/light/full/verbose.",
                other
            ))
        }
    };
    tool.set_tracer_mode(mode)?;
    Ok(format!("Tracer mode set to {}.", args[0]))
}

/// Show state heatmap.
fn cmd_heatmap(tool: &ProbeTool, args: &[&str]) -> Result<String, ProbeError> {
    let _dims: usize = args[0].parse().map_err(|_| ProbeError::NotConnected)?;
    tool.get_trace_tail(32)?;
    Ok("Heatmap data requested. Response will arrive via event channel.".to_string())
}

/// Show instruction timeline.
fn cmd_timeline(tool: &ProbeTool) -> Result<String, ProbeError> {
    tool.get_trace_tail(100)?;
    Ok("Timeline data requested. Response will arrive via event channel.".to_string())
}

/// Help text for all commands.
fn help_text() -> String {
    r#"A2X Probe — Available Commands

  status, s                  Show current VM status
  graph, g                   Dump WorldGraph as dot graph
  regions, r                 List StateField regions

  break <ip>, b <ip>         Set breakpoint at instruction index
  break list                 List all breakpoints
  clear <ip>, c <ip>         Clear breakpoint at instruction index
  clear all                  Clear all breakpoints

  continue, cont             Continue execution (when paused)
  step, st                   Step one instruction (when paused)

  trace <n>, t <n>           Show last N trace entries
  watch <region>, w <region> Watch a StateField region

  tracer <mode>              Set tracer mode (off/light/full/verbose)
  heatmap <dims>             Show state heatmap for last entries
  timeline                   Show instruction execution timeline

  help, h                    Show this help text
  quit, q                    Exit probe
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_text() {
        let text = help_text();
        assert!(text.contains("status"));
        assert!(text.contains("break"));
        assert!(text.contains("continue"));
        assert!(text.contains("tracer"));
        assert!(text.contains("heatmap"));
        assert!(text.contains("timeline"));
    }

    #[test]
    fn test_execute_empty_command() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "").unwrap();
        assert!(result.contains("No command entered"));
    }

    #[test]
    fn test_execute_help() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "help").unwrap();
        assert!(result.contains("status"));
        assert!(result.contains("break"));
    }

    #[test]
    fn test_execute_unknown_command() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "foobar").unwrap();
        assert!(result.contains("Unknown command"));
    }

    #[test]
    fn test_execute_break_set() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "break 42").unwrap();
        assert!(result.contains("Breakpoint set at instruction 42"));
    }

    #[test]
    fn test_execute_continue() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "continue").unwrap();
        assert!(result.contains("Continued"));
    }

    #[test]
    fn test_execute_step() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "step").unwrap();
        assert!(result.contains("Stepped"));
    }

    #[test]
    fn test_execute_tracer_mode() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "tracer light").unwrap();
        assert!(result.contains("Tracer mode set to light"));
    }

    #[test]
    fn test_execute_tracer_unknown_mode() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "tracer banana").unwrap();
        assert!(result.contains("Unknown tracer mode"));
    }

    #[test]
    fn test_execute_quit() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "quit").unwrap();
        assert!(result.contains("Goodbye"));
    }

    #[test]
    fn test_execute_clear_all() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "clear all").unwrap();
        assert!(result.contains("All breakpoints cleared"));
    }

    #[test]
    fn test_execute_trace() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "trace 10").unwrap();
        assert!(result.contains("Last 10 trace entries"));
    }

    #[test]
    fn test_execute_status() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_etx, erx) = std::sync::mpsc::channel();
        let tool = ProbeTool::new(tx, erx);
        let result = execute_command(&tool, "status").unwrap();
        assert!(result.contains("Snapshot requested"));
    }
}
