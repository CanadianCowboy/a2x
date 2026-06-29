// a2x-probe — Instruction tracer
// See plans/07-probe.md §6 (TracerMode) and §9 (Instruction tracer)
//
// The Tracer collects per-instruction log entries from the CcsVm and
// provides formatted output for the instruction tracer display.
// It reads entries from `CcsVm::tracer_log()` and formats them for
// CLI or file output.

use a2x_ccs::probe::{TraceLogEntry, TracerMode};

/// An instruction tracer that collects and formats trace log entries.
///
/// The tracer doesn't own the entries — it reads them from the VM's
/// `tracer_log()` slice and provides formatting/display logic.
pub struct Tracer {
    /// The tracer mode this tracer was configured with.
    mode: TracerMode,
}

impl Tracer {
    /// Create a new tracer with the given mode.
    pub fn new(mode: TracerMode) -> Self {
        Tracer { mode }
    }

    /// Get the tracer mode.
    pub fn mode(&self) -> TracerMode {
        self.mode
    }

    /// Format a single trace entry as a line of text.
    pub fn format_entry(entry: &TraceLogEntry) -> String {
        let trace_part = entry
            .trace_len
            .map(|tl| format!(" trace={}", tl))
            .unwrap_or_default();
        format!(
            "[{:4}] {:?} (step {:4}) state=[{:.3}..]{}",
            entry.ip,
            entry.opcode,
            entry.steps,
            entry.state_summary.first().unwrap_or(&0.0),
            trace_part,
        )
    }

    /// Format a slice of trace entries as a multi-line trace log.
    pub fn format_entries(entries: &[TraceLogEntry]) -> String {
        if entries.is_empty() {
            return "Trace log is empty.".to_string();
        }
        let lines: Vec<String> = entries.iter().map(Self::format_entry).collect();
        format!("Trace ({} entries):\n{}", entries.len(), lines.join("\n"))
    }

    /// Format the last N trace entries.
    pub fn format_tail(entries: &[TraceLogEntry], n: usize) -> String {
        let start = entries.len().saturating_sub(n);
        Self::format_entries(&entries[start..])
    }

    /// Generate a simple ASCII timeline of trace entries.
    ///
    /// Each line shows the instruction pointer position on a horizontal axis,
    /// useful for visualizing execution flow patterns.
    pub fn timeline(entries: &[TraceLogEntry]) -> String {
        if entries.is_empty() {
            return "Timeline is empty.".to_string();
        }
        let max_ip = entries.iter().map(|e| e.ip).max().unwrap_or(0);
        let width = (max_ip + 1).min(80); // cap at 80 columns
        let mut lines = Vec::new();
        lines.push(format!(
            "Instruction Trace Timeline ({} entries)",
            entries.len()
        ));
        lines.push("─".repeat(width + 12));

        // IP axis header
        let mut header = String::from("IP: ");
        for i in 0..width {
            if i % 10 == 0 {
                header.push_str(&format!("{:1}", i % 10));
            } else {
                header.push(' ');
            }
        }
        lines.push(header);

        // Mark execution positions
        let mut row = vec![' '; width];
        for entry in entries {
            if entry.ip < width {
                row[entry.ip] = '█';
            }
        }
        lines.push(format!("    {}", row.iter().collect::<String>()));

        // Show entry count per IP position
        let mut counts = vec![0u32; width];
        for entry in entries {
            if entry.ip < width {
                counts[entry.ip] += 1;
            }
        }
        let count_row: String = counts
            .iter()
            .map(|c| {
                if *c == 0 {
                    ' '
                } else if *c < 10 {
                    char::from_digit(*c, 10).unwrap_or(' ')
                } else {
                    '+'
                }
            })
            .collect();
        lines.push(format!("    {}", count_row));

        lines.join("\n")
    }

    /// Generate a heatmap of state field values across trace entries.
    ///
    /// Shows the first `dims` dimensions of the state summary for each
    /// trace entry as ASCII characters representing value ranges.
    pub fn state_heatmap(entries: &[TraceLogEntry], dims: usize) -> String {
        if entries.is_empty() {
            return "Heatmap is empty.".to_string();
        }

        let chars = " .:-=+*#%@";
        let mut lines = Vec::new();
        lines.push(format!(
            "State Heatmap ({} entries, {} dims shown)",
            entries.len(),
            dims
        ));
        lines.push("─".repeat(dims + 8));

        // Header
        let mut header = String::from("Step  ");
        for d in 0..dims {
            header.push_str(&format!("{:3}  ", d));
        }
        lines.push(header);

        for entry in entries.iter().take(64) {
            // Cap at 64 rows for readability
            let mut row = format!("{:4}  ", entry.steps);
            for d in 0..dims {
                if let Some(&val) = entry.state_summary.get(d) {
                    // Map [-1, 1] → [0, chars.len()-1]
                    let normalized = (val + 1.0) / 2.0;
                    let idx = (normalized * (chars.len() - 1) as f32)
                        .round()
                        .max(0.0)
                        .min((chars.len() - 1) as f32) as usize;
                    row.push_str(&format!("  {} ", chars.chars().nth(idx).unwrap_or('.')));
                } else {
                    row.push_str("  . ");
                }
            }
            lines.push(row);
        }

        if entries.len() > 64 {
            lines.push(format!("  ... ({} more entries)", entries.len() - 64));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::opcode::Opcode;

    fn make_entries(count: usize) -> Vec<TraceLogEntry> {
        (0..count)
            .map(|i| TraceLogEntry {
                ip: i % 10,
                opcode: match i % 3 {
                    0 => Opcode::Bind,
                    1 => Opcode::Ground,
                    _ => Opcode::Evolve,
                },
                steps: i,
                state_summary: vec![i as f32 * 0.1, -(i as f32 * 0.1), 0.5, -0.5],
                trace_len: Some(i + 1),
            })
            .collect()
    }

    #[test]
    fn test_format_entry() {
        let entries = make_entries(1);
        let text = Tracer::format_entry(&entries[0]);
        // Format is "[   0] Bind (step    0) state=[0.000..] trace=1"
        assert!(text.contains("Bind"));
        assert!(text.contains("step    0"));
        assert!(text.contains("state="));
        assert!(text.contains("trace="));
    }

    #[test]
    fn test_format_entries_empty() {
        let text = Tracer::format_entries(&[]);
        assert_eq!(text, "Trace log is empty.");
    }

    #[test]
    fn test_format_entries_multi() {
        let entries = make_entries(3);
        let text = Tracer::format_entries(&entries);
        assert!(text.contains("Trace (3 entries):"));
        assert!(text.contains("Bind"));
        assert!(text.contains("Ground"));
        assert!(text.contains("Evolve"));
    }

    #[test]
    fn test_format_tail() {
        let entries = make_entries(10);
        let text = Tracer::format_tail(&entries, 3);
        assert!(text.contains("Trace (3 entries):"));
    }

    #[test]
    fn test_timeline_empty() {
        let text = Tracer::timeline(&[]);
        assert_eq!(text, "Timeline is empty.");
    }

    #[test]
    fn test_timeline_basic() {
        let entries = make_entries(5);
        let text = Tracer::timeline(&entries);
        assert!(text.contains("Instruction Trace Timeline"));
        assert!(text.contains("IP:"));
        assert!(text.contains("█"));
    }

    #[test]
    fn test_state_heatmap_empty() {
        let text = Tracer::state_heatmap(&[], 4);
        assert_eq!(text, "Heatmap is empty.");
    }

    #[test]
    fn test_state_heatmap_basic() {
        let entries = make_entries(5);
        let text = Tracer::state_heatmap(&entries, 4);
        assert!(text.contains("State Heatmap"));
        assert!(text.contains("Step"));
    }

    #[test]
    fn test_state_heatmap_capped_at_64() {
        let entries = make_entries(100);
        let text = Tracer::state_heatmap(&entries, 2);
        assert!(text.contains("... (36 more entries)"));
    }
}
