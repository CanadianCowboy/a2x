// ContextMemory — persistent conversation memory and pattern extraction
//
// Like Copilot's working-set memory: tracks which files are accessed,
// which tools are used, and what topics were discussed — even after
// the original messages have been pruned from the context window.
//
// Pattern extraction scans tool calls, user/assistant messages, and
// tool results to build a condensed memory that persists across the
// entire conversation.

use std::collections::{HashMap, HashSet};

use crate::llm_backend::ChatMessage;

/// Approximate tokens per character for context window estimation.
pub(crate) const CHARS_PER_TOKEN: f64 = 4.0;

/// Safety margin: reserve 10% of the context window for the LLM's response.
pub(crate) const CONTEXT_SAFETY_MARGIN: f64 = 0.10;

/// Count of static system messages always preserved (prompt + tool desc).
pub(crate) const PRESERVED_SYSTEM_MSGS: usize = 2;

/// Token budget reserved for the dynamic memory summary message.
pub(crate) const MEMORY_TOKEN_BUDGET: u32 = 300;

/// Default context window size (32K tokens — common for 7B models).
pub(crate) const DEFAULT_CONTEXT_TOKENS: u32 = 32768;

// ── Context Memory ────────────────────────────────────────────────────────

/// Persistent conversation memory that accumulates patterns across turns.
///
/// Like Copilot's working-set memory: tracks which files are accessed,
/// which tools are used, and what topics were discussed — even after
/// the original messages have been pruned from the context window.
#[derive(Clone, Debug, Default)]
pub struct ContextMemory {
    /// File paths that have been read or written (deduplicated).
    pub accessed_files: HashSet<String>,
    /// Tool usage counts (tool name → invocation count).
    pub tool_usage: HashMap<String, usize>,
    /// Key topics extracted from user messages (most recent first).
    pub topics: Vec<String>,
    /// Notable decisions or outcomes from the conversation.
    pub decisions: Vec<String>,
    /// Most recent working directory mentioned.
    pub working_dir: Option<String>,
    /// Total messages processed into memory.
    pub messages_processed: u64,
}

impl ContextMemory {
    /// Produce a compact system-message summary of accumulated memory.
    /// Fits within MEMORY_TOKEN_BUDGET.
    pub fn to_system_message(&self) -> Option<ChatMessage> {
        if self.is_empty() {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();

        // Files section (most important — like Copilot's #file references)
        if !self.accessed_files.is_empty() {
            let mut files: Vec<&String> = self.accessed_files.iter().collect();
            files.sort();
            let file_list: Vec<String> = files.iter().take(8).map(|f| format!("`{}`", f)).collect();
            let suffix = if files.len() > 8 {
                format!(" (+{} more)", files.len() - 8)
            } else {
                String::new()
            };
            parts.push(format!(
                "**Working files:** {}{}",
                file_list.join(", "),
                suffix
            ));
        }

        // Working directory
        if let Some(ref dir) = self.working_dir {
            parts.push(format!("**Working directory:** `{}`", dir));
        }

        // Tool patterns
        if !self.tool_usage.is_empty() {
            let mut tools: Vec<(&String, &usize)> = self.tool_usage.iter().collect();
            tools.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            let tool_list: Vec<String> = tools
                .iter()
                .take(5)
                .map(|(name, count)| format!("{}({}×)", name, count))
                .collect();
            parts.push(format!("**Frequent tools:** {}", tool_list.join(", ")));
        }

        // Key topics (most recent first — what we were just discussing)
        if !self.topics.is_empty() {
            let recent_topics: Vec<&str> = self.topics.iter().take(5).map(|s| s.as_str()).collect();
            parts.push(format!("**Recent topics:** {}", recent_topics.join(" → ")));
        }

        // Important decisions
        if !self.decisions.is_empty() {
            let recent_decisions: Vec<&str> = self
                .decisions
                .iter()
                .rev()
                .take(3)
                .map(|s| s.as_str())
                .collect();
            parts.push(format!("**Decisions:** {}", recent_decisions.join("; ")));
        }

        let content = format!("[Context Memory]\n{}", parts.join("\n"));
        Some(ChatMessage::system(content))
    }

    /// Whether the memory is completely empty (no patterns accumulated).
    pub fn is_empty(&self) -> bool {
        self.accessed_files.is_empty()
            && self.tool_usage.is_empty()
            && self.topics.is_empty()
            && self.decisions.is_empty()
            && self.working_dir.is_none()
    }
}

// ── Pattern Extraction ────────────────────────────────────────────────────

/// Extract patterns from a single chat message and update memory.
pub fn extract_message_patterns(msg: &ChatMessage, memory: &mut ContextMemory) {
    // Scan for file paths in content (common patterns: /path, C:\, ./, ~/)
    scan_for_paths(&msg.content, memory);

    // Scan tool call arguments for paths and record tool usage
    for tc in &msg.tool_calls {
        memory
            .tool_usage
            .entry(tc.name.clone())
            .and_modify(|c| *c += 1)
            .or_insert(1);

        // Parse tool arguments to find paths
        if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.arguments) {
            if let Some(path) = args["path"].as_str() {
                memory.accessed_files.insert(path.to_string());
                // Detect working directory
                if let Some(parent) = std::path::Path::new(path).parent() {
                    if parent != std::path::Path::new("") && parent != std::path::Path::new(".") {
                        memory.working_dir = Some(parent.display().to_string());
                    }
                }
            }
        }
    }

    // Extract topic keywords from user/assistant messages
    if !msg.content.is_empty() && msg.content.len() < 2000 {
        extract_topics(&msg.content, memory);
    }

    memory.messages_processed += 1;
}

/// Naive file path scanner — looks for common path patterns in text.
pub fn scan_for_paths(text: &str, memory: &mut ContextMemory) {
    for word in text.split_whitespace() {
        let trimmed =
            word.trim_matches(|c: char| c == ',' || c == ';' || c == '"' || c == '\'' || c == '`');
        if trimmed.is_empty() || trimmed == "." {
            continue;
        }

        // Common path indicators
        let looks_like_path = trimmed.starts_with('/') ||
            trimmed.starts_with("./") ||
            trimmed.starts_with("~/") ||
            trimmed.starts_with("..") ||
            // Windows: C:\, D:\, etc. (colon at position 1, backslash at position 2)
            (trimmed.len() >= 3
                && trimmed.as_bytes().get(1).copied() == Some(b':')
                && trimmed.as_bytes().get(2).copied() == Some(b'\\')) ||
            trimmed.contains('/') && trimmed.split('/').count() >= 2;

        if looks_like_path && trimmed.len() <= 256 {
            memory.accessed_files.insert(trimmed.to_string());
        }
    }
}

/// Simple topic extraction — picks significant words and phrases.
/// Extracts: quoted strings, CapitalizedWords, and significant noun phrases.
pub fn extract_topics(text: &str, memory: &mut ContextMemory) {
    // Extract key noun phrases: capitalized words, quoted strings, code identifiers
    let mut found: Vec<String> = Vec::new();

    // Quoted strings often indicate topics (split on both double and single quotes)
    for segment in text.split(['"', '\'']) {
        let w = segment.trim();
        if w.len() >= 4 && w.len() <= 60 && !w.starts_with("http") {
            found.push(w.to_string());
        }
        // Also check for backtick-quoted identifiers
        if w.contains('`') {
            for part in w.split('`') {
                let p = part.trim();
                if p.len() >= 3 && p.len() <= 40 && !p.contains(' ') {
                    found.push(p.to_string());
                }
            }
        }
    }

    // Capitalized words (potential proper nouns / topics)
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric());
        if w.len() >= 4
            && w.chars().next().is_some_and(|c| c.is_uppercase())
            && !w.bytes().all(|b| b.is_ascii_uppercase())
        // skip ALL CAPS
        {
            found.push(w.to_string());
        }
    }

    // Deduplicate and add to topics (most recent first, capped at 20)
    found.sort();
    found.dedup();
    for topic in found.into_iter().take(5) {
        if !memory.topics.contains(&topic) {
            memory.topics.push(topic);
            if memory.topics.len() > 20 {
                memory.topics.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_backend::ToolCall;

    // ── ContextMemory tests ───────────────────────────────────────────

    #[test]
    fn test_context_memory_empty() {
        let mem = ContextMemory::default();
        assert!(mem.is_empty());
        assert!(mem.to_system_message().is_none());
    }

    #[test]
    fn test_context_memory_with_files() {
        let mut mem = ContextMemory::default();
        mem.accessed_files.insert("/src/main.rs".into());
        mem.accessed_files.insert("/src/lib.rs".into());
        let msg = mem.to_system_message().unwrap();
        let content = &msg.content;
        assert!(content.contains("main.rs"));
        assert!(content.contains("lib.rs"));
        assert!(content.contains("Working files"));
    }

    #[test]
    fn test_context_memory_with_tools() {
        let mut mem = ContextMemory::default();
        mem.tool_usage.insert("shell_exec".into(), 5);
        mem.tool_usage.insert("fs_read".into(), 3);
        let msg = mem.to_system_message().unwrap();
        assert!(msg.content.contains("shell_exec(5×)"));
        assert!(msg.content.contains("fs_read(3×)"));
    }

    #[test]
    fn test_context_memory_with_topics() {
        let mut mem = ContextMemory::default();
        mem.topics.push("Authentication".into());
        mem.topics.push("Database".into());
        let msg = mem.to_system_message().unwrap();
        assert!(msg.content.contains("Authentication"));
        assert!(msg.content.contains("Database"));
    }

    #[test]
    fn test_context_memory_with_working_dir() {
        let mem = ContextMemory {
            working_dir: Some("/home/user/project".into()),
            ..Default::default()
        };
        let msg = mem.to_system_message().unwrap();
        assert!(msg.content.contains("/home/user/project"));
    }

    #[test]
    fn test_context_memory_with_decisions() {
        let mut mem = ContextMemory::default();
        mem.decisions.push("Use sqlite".into());
        mem.decisions.push("Port 8080".into());
        let msg = mem.to_system_message().unwrap();
        assert!(msg.content.contains("sqlite"));
    }

    #[test]
    fn test_context_memory_truncates_files_to_8() {
        let mut mem = ContextMemory::default();
        for i in 0..15 {
            mem.accessed_files.insert(format!("/file_{}.rs", i));
        }
        let msg = mem.to_system_message().unwrap();
        assert!(msg.content.contains("(+7 more)"));
    }

    #[test]
    fn test_context_memory_truncates_tools_to_5() {
        let mut mem = ContextMemory::default();
        for i in 0..10 {
            mem.tool_usage.insert(format!("tool_{}", i), i);
        }
        let msg = mem.to_system_message().unwrap();
        // Should show top 5 tools only
        let tool_count = msg.content.matches('×').count();
        assert!(tool_count <= 5);
    }

    // ── scan_for_paths tests ───────────────────────────────────────────

    #[test]
    fn test_scan_for_paths_unix() {
        let mut mem = ContextMemory::default();
        scan_for_paths("check /home/user/src/main.rs and ./lib.rs", &mut mem);
        assert!(mem.accessed_files.contains("/home/user/src/main.rs"));
        assert!(mem.accessed_files.contains("./lib.rs"));
    }

    #[test]
    fn test_scan_for_paths_windows() {
        let mut mem = ContextMemory::default();
        scan_for_paths(
            "run C:\\Users\\test\\file.txt and D:\\projects\\app.rs",
            &mut mem,
        );
        assert!(mem.accessed_files.contains("C:\\Users\\test\\file.txt"));
        assert!(mem.accessed_files.contains("D:\\projects\\app.rs"));
    }

    #[test]
    fn test_scan_for_paths_home_dir() {
        let mut mem = ContextMemory::default();
        scan_for_paths(
            "edit ~/.config/nvim/init.lua and ~/projects/main.rs",
            &mut mem,
        );
        assert!(mem.accessed_files.contains("~/.config/nvim/init.lua"));
        assert!(mem.accessed_files.contains("~/projects/main.rs"));
    }

    #[test]
    fn test_scan_for_paths_parent_dir() {
        let mut mem = ContextMemory::default();
        scan_for_paths("include ../common/utils.rs", &mut mem);
        assert!(mem.accessed_files.contains("../common/utils.rs"));
    }

    #[test]
    fn test_scan_for_paths_backtick_quoted() {
        let mut mem = ContextMemory::default();
        scan_for_paths("the file `/etc/nginx/nginx.conf` needs updating", &mut mem);
        assert!(mem.accessed_files.contains("/etc/nginx/nginx.conf"));
    }

    #[test]
    fn test_scan_for_paths_double_quoted() {
        let mut mem = ContextMemory::default();
        scan_for_paths("open \"/var/log/syslog\" for reading", &mut mem);
        assert!(mem.accessed_files.contains("/var/log/syslog"));
    }

    #[test]
    fn test_scan_for_paths_with_trailing_comma() {
        let mut mem = ContextMemory::default();
        scan_for_paths("import /src/models/user.rs, /src/models/post.rs,", &mut mem);
        assert!(mem.accessed_files.contains("/src/models/user.rs"));
        assert!(mem.accessed_files.contains("/src/models/post.rs"));
    }

    #[test]
    fn test_scan_for_paths_deduplicates() {
        let mut mem = ContextMemory::default();
        scan_for_paths("/src/main.rs and also /src/main.rs", &mut mem);
        assert_eq!(mem.accessed_files.len(), 1);
    }

    #[test]
    fn test_scan_for_paths_ignores_non_paths() {
        let mut mem = ContextMemory::default();
        scan_for_paths("hello world this is a test with numbers 123", &mut mem);
        assert!(mem.accessed_files.is_empty());
    }

    #[test]
    fn test_scan_for_paths_ignores_single_dot() {
        let mut mem = ContextMemory::default();
        scan_for_paths("run . now", &mut mem);
        assert!(mem.accessed_files.is_empty());
    }

    #[test]
    fn test_scan_for_paths_rejects_long_paths() {
        let mut mem = ContextMemory::default();
        let long_path = "/".to_string() + &"a".repeat(300);
        scan_for_paths(&long_path, &mut mem);
        assert!(mem.accessed_files.is_empty());
    }

    #[test]
    fn test_scan_for_paths_deeply_nested() {
        let mut mem = ContextMemory::default();
        scan_for_paths("check /a/b/c/d/e/f/g/deep.rs", &mut mem);
        assert!(mem.accessed_files.contains("/a/b/c/d/e/f/g/deep.rs"));
    }

    // ── extract_topics tests ───────────────────────────────────────────

    #[test]
    fn test_extract_topics_capitalized() {
        let mut mem = ContextMemory::default();
        extract_topics(
            "I need to refactor the Authentication module and fix the Database connection",
            &mut mem,
        );
        assert!(mem.topics.iter().any(|t| t.contains("Authentication")));
        assert!(mem.topics.iter().any(|t| t.contains("Database")));
    }

    #[test]
    fn test_extract_topics_from_quotes() {
        let mut mem = ContextMemory::default();
        extract_topics(
            "The \"UserService\" needs to handle \"rate limiting\" properly",
            &mut mem,
        );
        assert!(mem.topics.iter().any(|t| t == "UserService"));
        assert!(mem.topics.iter().any(|t| t == "rate limiting"));
    }

    #[test]
    fn test_extract_topics_from_single_quotes() {
        let mut mem = ContextMemory::default();
        extract_topics(
            "call the 'AuthProvider' and 'TokenManager' classes",
            &mut mem,
        );
        assert!(mem.topics.iter().any(|t| t == "AuthProvider"));
        assert!(mem.topics.iter().any(|t| t == "TokenManager"));
    }

    #[test]
    fn test_extract_topics_from_backticks() {
        let mut mem = ContextMemory::default();
        extract_topics("the `UserService` struct calls `handle_request`", &mut mem);
        assert!(mem.topics.iter().any(|t| t == "UserService"));
        assert!(mem.topics.iter().any(|t| t == "handle_request"));
    }

    #[test]
    fn test_extract_topics_skip_all_caps() {
        let mut mem = ContextMemory::default();
        // ALL CAPS acronyms like "API" or "HTTP" should be skipped
        extract_topics("the HTTP API needs HTTPS support", &mut mem);
        // "HTTP" is 4 chars but all caps, so skipped. "API" is 3 chars, skipped.
        // But "HTTPS" is 5 chars and all caps, so skipped too.
        assert!(!mem.topics.iter().any(|t| t == "HTTP"));
        assert!(!mem.topics.iter().any(|t| t == "HTTPS"));
    }

    #[test]
    fn test_extract_topics_skip_short_words() {
        let mut mem = ContextMemory::default();
        extract_topics("The API is OK now", &mut mem);
        // "API" is 3 chars (< 4), "OK" is 2 chars (< 4)
        assert!(mem.topics.is_empty() || mem.topics.iter().all(|t| t.len() >= 4));
    }

    #[test]
    fn test_extract_topics_skip_urls() {
        let mut mem = ContextMemory::default();
        extract_topics("see https://docs.rs/tokio for details", &mut mem);
        assert!(!mem.topics.iter().any(|t| t.starts_with("http")));
    }

    #[test]
    fn test_extract_topics_deduplicates() {
        let mut mem = ContextMemory::default();
        extract_topics("the Database and the Database module", &mut mem);
        let db_count = mem.topics.iter().filter(|t| *t == "Database").count();
        assert_eq!(db_count, 1);
    }

    #[test]
    fn test_extract_topics_capped_at_20() {
        let mut mem = ContextMemory::default();
        // Feed 25 unique topics via extract_topics — the function caps at 20
        for i in 0..25 {
            extract_topics(&format!("Topic{}", i), &mut mem);
        }
        // After capping, only last 20 should remain (oldest 5 removed)
        assert_eq!(mem.topics.len(), 20);
        assert_eq!(mem.topics[0], "Topic5");
        assert_eq!(mem.topics[19], "Topic24");
    }

    // ── extract_message_patterns tests ─────────────────────────────────

    #[test]
    fn test_extract_message_patterns_with_tool_calls() {
        let mut mem = ContextMemory::default();
        let tc = ToolCall {
            id: "call_1".into(),
            name: "fs_read".into(),
            arguments: r#"{"path": "/home/user/config.toml", "max_lines": 100}"#.into(),
        };
        let mut msg = ChatMessage::assistant("");
        msg.tool_calls = vec![tc];
        extract_message_patterns(&msg, &mut mem);

        assert_eq!(mem.tool_usage.get("fs_read"), Some(&1));
        assert!(mem.accessed_files.contains("/home/user/config.toml"));
        assert_eq!(mem.working_dir, Some("/home/user".into()));
    }

    #[test]
    fn test_extract_message_patterns_with_tool_no_path() {
        let mut mem = ContextMemory::default();
        let tc = ToolCall {
            id: "call_2".into(),
            name: "list_agents".into(),
            arguments: r#"{}"#.into(),
        };
        let mut msg = ChatMessage::assistant("");
        msg.tool_calls = vec![tc];
        extract_message_patterns(&msg, &mut mem);

        assert_eq!(mem.tool_usage.get("list_agents"), Some(&1));
        assert!(mem.accessed_files.is_empty());
    }

    #[test]
    fn test_extract_message_patterns_multiple_tools() {
        let mut mem = ContextMemory::default();
        let tc1 = ToolCall {
            id: "c1".into(),
            name: "shell_exec".into(),
            arguments: r#"{"command": "ls"}"#.into(),
        };
        let tc2 = ToolCall {
            id: "c2".into(),
            name: "fs_read".into(),
            arguments: r#"{"path": "/src/lib.rs"}"#.into(),
        };
        let mut msg = ChatMessage::assistant("");
        msg.tool_calls = vec![tc1, tc2];
        extract_message_patterns(&msg, &mut mem);

        assert_eq!(mem.tool_usage.get("shell_exec"), Some(&1));
        assert_eq!(mem.tool_usage.get("fs_read"), Some(&1));
        assert!(mem.accessed_files.contains("/src/lib.rs"));
    }

    #[test]
    fn test_extract_message_patterns_increments_counts() {
        let mut mem = ContextMemory::default();
        let tc = ToolCall {
            id: "c1".into(),
            name: "shell_exec".into(),
            arguments: r#"{"command": "ls"}"#.into(),
        };
        let mut msg = ChatMessage::assistant("");
        msg.tool_calls = vec![tc.clone()];
        extract_message_patterns(&msg, &mut mem);

        let mut msg2 = ChatMessage::assistant("");
        msg2.tool_calls = vec![tc];
        extract_message_patterns(&msg2, &mut mem);

        assert_eq!(mem.tool_usage.get("shell_exec"), Some(&2));
    }

    #[test]
    fn test_extract_message_patterns_increments_messages_processed() {
        let mut mem = ContextMemory::default();
        let msg = ChatMessage::user("hello");
        extract_message_patterns(&msg, &mut mem);
        assert_eq!(mem.messages_processed, 1);

        extract_message_patterns(&msg, &mut mem);
        assert_eq!(mem.messages_processed, 2);
    }

    #[test]
    fn test_extract_message_patterns_skips_long_messages_for_topics() {
        let mut mem = ContextMemory::default();
        let long_text = "x".repeat(2500);
        let msg = ChatMessage::user(&long_text);
        // Should not panic and should still process paths/tools
        extract_message_patterns(&msg, &mut mem);
        assert_eq!(mem.messages_processed, 1);
    }
}
