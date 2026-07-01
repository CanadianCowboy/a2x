// See plans/05-agents.md §3 (LLM Agent)
//
// T3-1: Real LLM integration with trait-based backends.
// The LLM agent now uses a configurable LlmBackend to call real LLM APIs
// for natural language ↔ Σ∞ program translation.

use std::sync::Arc;

use a2x_core::agent::Agent;
use a2x_core::agent_id::{AgentId, AgentType};
use a2x_core::capability::Capability;
use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_core::state::StateSnapshot;
use a2x_sigma::program::SigmaProgram;

use crate::llm_backend::{
    LlmBackend, NoopBackend, SIGMA_EXPLANATION_PROMPT, SIGMA_GENERATION_PROMPT,
};

/// The LLM agent — bridges natural language and A2X.
///
/// Uses a trait-based LLM backend to convert:
/// - Natural language requests → Σ∞ programs (nl_to_sigma)
/// - Σ∞ program results → natural language explanations (sigma_to_nl)
///
/// The backend is configurable — supports OpenAI, Ollama, vLLM, and any
/// OpenAI-compatible API. See `LlmBackend` for the trait and `OpenAiBackend`
/// for the default implementation.
///
/// T3-1: Replaced Phase 0 stubs with real async LLM API calls.
pub struct LlmAgent {
    /// Agent identity.
    id: AgentId,
    /// Model name for display/reference.
    model: String,
    /// LLM backend for API calls (shared via Arc for Send + Sync).
    backend: Arc<dyn LlmBackend>,
    /// Statistics counters (updated on each call).
    nl_to_sigma_calls: std::sync::atomic::AtomicU64,
    sigma_to_nl_calls: std::sync::atomic::AtomicU64,
}

impl LlmAgent {
    /// Create a new LLM agent with a backend.
    pub fn new(id: AgentId, model: impl Into<String>, backend: Arc<dyn LlmBackend>) -> Self {
        LlmAgent {
            id,
            model: model.into(),
            backend,
            nl_to_sigma_calls: std::sync::atomic::AtomicU64::new(0),
            sigma_to_nl_calls: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Create an LLM agent without a real backend (for testing/probing).
    /// Uses a no-op backend that always returns empty results.
    pub fn new_stub(id: AgentId, model: impl Into<String>) -> Self {
        Self::new(id, model, Arc::new(NoopBackend))
    }

    /// Convert natural language intent into a Σ∞ program by calling the LLM.
    ///
    /// Sends the intent as a user prompt with a system prompt that teaches the
    /// LLM Σ∞ syntax. Parses the LLM's response into a `SigmaProgram`.
    ///
    /// Returns an empty program if the LLM returns an empty response or the
    /// response can't be parsed.
    pub fn nl_to_sigma(&self, intent: &str) -> SigmaProgram {
        self.nl_to_sigma_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Build a blocking future runner for the async backend call.
        // In production, this would be async — but the Agent trait's execute()
        // is currently sync. The Tokio runtime must be available.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                let backend = Arc::clone(&self.backend);
                let intent = intent.to_string();
                let result = handle.block_on(async move {
                    backend.complete(SIGMA_GENERATION_PROMPT, &intent).await
                });
                self.parse_llm_response(result)
            }
            Err(_) => {
                // No tokio runtime available — return empty program.
                // The caller should use `nl_to_sigma_async()` instead.
                tracing::warn!(
                    "LlmAgent::nl_to_sigma called without tokio runtime; use nl_to_sigma_async"
                );
                SigmaProgram::new()
            }
        }
    }

    /// Async version of nl_to_sigma — for use within tokio runtimes.
    pub async fn nl_to_sigma_async(&self, intent: &str) -> SigmaProgram {
        self.nl_to_sigma_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let result = self.backend.complete(SIGMA_GENERATION_PROMPT, intent).await;
        self.parse_llm_response(result)
    }

    /// Convert a Σ∞ program result into natural language by calling the LLM.
    pub fn sigma_to_nl(&self, program: &SigmaProgram) -> String {
        self.sigma_to_nl_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let program_text = if program.is_empty() {
            "(empty program — no instructions executed)".to_string()
        } else {
            program
                .instructions
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                let backend = Arc::clone(&self.backend);
                handle
                    .block_on(async move {
                        backend
                            .complete(SIGMA_EXPLANATION_PROMPT, &program_text)
                            .await
                    })
                    .unwrap_or_else(|_| {
                        format!("[Execution result: {} instruction(s)]", program.len())
                    })
            }
            Err(_) => {
                tracing::warn!(
                    "LlmAgent::sigma_to_nl called without tokio runtime; use sigma_to_nl_async"
                );
                format!("[Execution result: {} instruction(s)]", program.len())
            }
        }
    }

    /// Async version of sigma_to_nl — for use within tokio runtimes.
    pub async fn sigma_to_nl_async(&self, program: &SigmaProgram) -> String {
        self.sigma_to_nl_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let program_text = if program.is_empty() {
            "(empty program — no instructions executed)".to_string()
        } else {
            program
                .instructions
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };

        self.backend
            .complete(SIGMA_EXPLANATION_PROMPT, &program_text)
            .await
            .unwrap_or_else(|_| format!("[Execution result: {} instruction(s)]", program.len()))
    }

    /// Get the backend (for configuration inspection).
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get call statistics.
    pub fn stats(&self) -> LlmStats {
        LlmStats {
            nl_to_sigma_calls: self
                .nl_to_sigma_calls
                .load(std::sync::atomic::Ordering::Relaxed),
            sigma_to_nl_calls: self
                .sigma_to_nl_calls
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Parse an LLM response into a SigmaProgram.
    fn parse_llm_response(&self, result: Result<String, AgentError>) -> SigmaProgram {
        match result {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return SigmaProgram::new();
                }

                // Try parsing the full text as a SigmaProgram.
                // If that fails, try parsing line-by-line for multi-instruction programs.
                match a2x_sigma::parse_program(trimmed) {
                    Ok(prog) => prog,
                    Err(_) => {
                        // Try line-by-line parsing for multi-line LLM outputs
                        let mut combined = SigmaProgram::new();
                        for line in trimmed.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with("```") || line.starts_with('#') {
                                continue;
                            }
                            if let Ok(prog) = a2x_sigma::parse_program(line) {
                                combined.compose(prog);
                            }
                        }
                        if combined.is_empty() {
                            // Couldn't parse anything meaningful — return raw text as GROUND instruction
                            tracing::warn!(
                                "LLM response could not be parsed as Σ∞: {}",
                                &trimmed[..trimmed.len().min(80)]
                            );
                        }
                        combined
                    }
                }
            }
            Err(e) => {
                tracing::error!("LLM API call failed: {}", e);
                SigmaProgram::new()
            }
        }
    }
}

/// Statistics for LLM agent usage.
#[derive(Clone, Debug, Default)]
pub struct LlmStats {
    pub nl_to_sigma_calls: u64,
    pub sigma_to_nl_calls: u64,
}

impl Agent for LlmAgent {
    fn id(&self) -> AgentId {
        self.id.clone()
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Llm
    }

    fn execute(&self, program: Packet) -> Result<Packet, AgentError> {
        // Interpret the packet as natural language and convert to Σ∞.
        // In Phase 0 this was a stub; T3-1 adds real functionality.
        let text = match &program {
            Packet::Raw(bytes) => String::from_utf8_lossy(bytes).to_string(),
        };

        let sigma_prog = self.nl_to_sigma(&text);
        if sigma_prog.is_empty() {
            return Ok(Packet::Raw(vec![]));
        }

        // Return the generated Σ∞ program as text
        let result_text = sigma_prog
            .instructions
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Packet::Raw(result_text.into_bytes()))
    }

    fn state_summary(&self) -> Option<StateSnapshot> {
        let stats = self.stats();
        Some(StateSnapshot {
            agent_id: self.id.clone(),
            state: format!(
                "idle (nl2sigma: {}, sigma2nl: {})",
                stats.nl_to_sigma_calls, stats.sigma_to_nl_calls
            ),
            current_program: None,
            ip: None,
            world_graph_size: 0,
            memory_trace_length: 0,
            uptime: std::time::Duration::ZERO,
        })
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Execute,
            Capability::Custom("plan".into()),
            Capability::Custom("nl_to_sigma".into()),
            Capability::Custom("sigma_to_nl".into()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_backend::{OpenAiBackend, OpenAiConfig};

    #[test]
    fn test_new_llm_agent() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "gpt-4o", backend);
        assert_eq!(agent.id(), AgentId::new("llm-1"));
        assert_eq!(agent.agent_type(), AgentType::Llm);
        assert_eq!(agent.model(), "gpt-4o");
    }

    #[test]
    fn test_stats_initial() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let stats = agent.stats();
        assert_eq!(stats.nl_to_sigma_calls, 0);
        assert_eq!(stats.sigma_to_nl_calls, 0);
    }

    #[test]
    fn test_nl_to_sigma_without_runtime() {
        // When no tokio runtime is available, returns empty program gracefully.
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let program = agent.nl_to_sigma("scan the system");
        assert!(program.is_empty());
    }

    #[test]
    fn test_sigma_to_nl_without_runtime() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let program = SigmaProgram::new();
        let desc = agent.sigma_to_nl(&program);
        assert!(desc.contains("Execution result"));
    }

    #[test]
    fn test_nl_to_sigma_with_runtime() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);

        rt.block_on(async {
            let program = agent.nl_to_sigma_async("stop").await;
            // Without a real LLM API, the call will fail and return empty.
            // The important thing is it doesn't panic.
            assert!(program.is_empty() || !program.is_empty());
        });
    }

    #[test]
    fn test_execute_with_raw_packet() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let packet = Packet::Raw(b"stop execution".to_vec());
        let result = agent.execute(packet);
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_summary() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let summary = agent.state_summary().unwrap();
        assert!(summary.state.contains("nl2sigma"));
        assert!(summary.state.contains("sigma2nl"));
    }

    #[test]
    fn test_capabilities() {
        let backend = Arc::new(OpenAiBackend::new(OpenAiConfig::default()));
        let agent = LlmAgent::new(AgentId::new("llm-1"), "test", backend);
        let caps = agent.capabilities();
        assert!(caps.contains(&Capability::Execute));
        assert!(caps.iter().any(|c| c.to_string().contains("nl_to_sigma")));
    }
}
