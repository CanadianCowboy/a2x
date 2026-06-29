// See plans/06-entity-gateway.md §5 (stdin/stdout Listener)
//
// For CLI/pipe integration:
//   echo "⟦Σ∞⟧⟬I:✦ ∷ C:⟨sys⟩ ∷ P:⥅ ∷ D:⌵⟭" | a2x-gateway --listen stdio
//
// Reads one Σ∞ program per line from stdin, executes it, prints the result.

use std::io::{self, BufRead, Write};

use crate::entity::EntityId;
use crate::error::GatewayError;
use crate::listeners::{IncomingMessage, OutgoingMessage, ProtocolListener, ProtocolListenerType};

/// stdin/stdout protocol listener.
///
/// Reads Σ∞ programs line-by-line from stdin, sends them to the gateway,
/// and writes results to stdout.
pub struct StdioListener {
    running: bool,
    entity_id: EntityId,
}

impl StdioListener {
    pub fn new(entity_id: impl Into<String>) -> Self {
        StdioListener {
            running: false,
            entity_id: EntityId::new(entity_id),
        }
    }

    /// Process one line from stdin.
    ///
    /// Returns an `IncomingMessage` if the line is a valid Σ∞ program,
    /// or `None` for blank lines / comments.
    pub fn process_line(&self, line: &str, correlation_id: u64) -> Option<IncomingMessage> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        Some(IncomingMessage {
            entity_id: self.entity_id.clone(),
            payload: trimmed.as_bytes().to_vec(),
            correlation_id,
            is_text: true,
        })
    }

    /// Format an outgoing message for stdout display.
    pub fn format_output(msg: &OutgoingMessage) -> Option<String> {
        if msg.is_text {
            String::from_utf8(msg.payload.clone()).ok()
        } else {
            // Binary output: hex encode
            Some(
                msg.payload
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>(),
            )
        }
    }
}

impl ProtocolListener for StdioListener {
    fn listener_type(&self) -> ProtocolListenerType {
        ProtocolListenerType::Stdio
    }

    fn start(&mut self) -> Result<(), GatewayError> {
        self.running = true;
        tracing::info!("stdio listener started (entity: {})", self.entity_id);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), GatewayError> {
        self.running = false;
        tracing::info!("stdio listener stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn bound_address(&self) -> Option<String> {
        None // stdio has no network address
    }
}

/// Run a synchronous stdio loop: read from stdin, process, write to stdout.
///
/// This blocks until stdin is closed (EOF). Used by the gateway's stdio mode.
pub fn run_stdio_loop<F>(entity_id: &str, mut handler: F) -> Result<(), GatewayError>
where
    F: FnMut(IncomingMessage) -> Option<OutgoingMessage>,
{
    let listener = StdioListener::new(entity_id);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut correlation_id: u64 = 0;

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| GatewayError::Transport(e.to_string()))?;
        if let Some(msg) = listener.process_line(&line, correlation_id) {
            correlation_id = correlation_id.wrapping_add(1);
            if let Some(response) = handler(msg) {
                if let Some(text) = StdioListener::format_output(&response) {
                    writeln!(stdout, "{}", text)
                        .map_err(|e| GatewayError::Transport(e.to_string()))?;
                    stdout
                        .flush()
                        .map_err(|e| GatewayError::Transport(e.to_string()))?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_line_valid() {
        let listener = StdioListener::new("stdio-1");
        let input = "⟦Σ∞⟧⟬I:✕⟭";
        let msg = listener.process_line(input, 0).unwrap();
        assert!(msg.is_text);
        assert_eq!(msg.payload, input.as_bytes());
    }

    #[test]
    fn test_process_line_empty() {
        let listener = StdioListener::new("stdio-1");
        assert!(listener.process_line("", 0).is_none());
        assert!(listener.process_line("  ", 0).is_none());
    }

    #[test]
    fn test_process_line_comment() {
        let listener = StdioListener::new("stdio-1");
        assert!(listener.process_line("# this is a comment", 0).is_none());
    }

    #[test]
    fn test_format_output_text() {
        let msg = OutgoingMessage {
            entity_id: EntityId::new("stdio-1"),
            payload: b"result".to_vec(),
            correlation_id: 0,
            is_text: true,
        };
        assert_eq!(StdioListener::format_output(&msg), Some("result".into()));
    }

    #[test]
    fn test_format_output_binary() {
        let msg = OutgoingMessage {
            entity_id: EntityId::new("stdio-1"),
            payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
            correlation_id: 0,
            is_text: false,
        };
        assert_eq!(StdioListener::format_output(&msg), Some("deadbeef".into()));
    }

    #[test]
    fn test_listener_lifecycle() {
        let mut listener = StdioListener::new("stdio-1");
        assert!(!listener.is_running());
        listener.start().unwrap();
        assert!(listener.is_running());
        assert!(listener.bound_address().is_none());
        listener.stop().unwrap();
        assert!(!listener.is_running());
    }
}
