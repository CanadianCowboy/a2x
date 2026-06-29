// See plans/06-entity-gateway.md §5 (webhook callback)
// Webhook manager for async result delivery to external entities.

use std::collections::HashMap;

use crate::entity::EntityId;

/// A registered webhook entry.
#[derive(Clone, Debug)]
pub struct WebhookEntry {
    /// The entity that registered this webhook.
    pub entity_id: EntityId,
    /// The URL to POST results to.
    pub url: String,
    /// Optional: only fire for these correlation IDs.
    pub filter_correlation_ids: Option<Vec<u64>>,
}

/// Manages webhook registrations and delivers callbacks.
pub struct WebhookManager {
    /// Webhook entries indexed by a unique webhook ID.
    webhooks: HashMap<String, WebhookEntry>,
    /// Counter for generating webhook IDs.
    next_id: u64,
}

impl WebhookManager {
    pub fn new() -> Self {
        WebhookManager {
            webhooks: HashMap::new(),
            next_id: 0,
        }
    }

    /// Register a webhook. Returns the webhook ID.
    pub fn register(&mut self, entry: WebhookEntry) -> String {
        let id = format!("wh-{}", self.next_id);
        self.next_id += 1;
        self.webhooks.insert(id.clone(), entry);
        id
    }

    /// Remove a webhook by ID.
    pub fn unregister(&mut self, id: &str) -> bool {
        self.webhooks.remove(id).is_some()
    }

    /// Remove all webhooks for an entity.
    pub fn unregister_entity(&mut self, entity_id: &EntityId) {
        self.webhooks
            .retain(|_, entry| entry.entity_id != *entity_id);
    }

    /// Get all webhooks that should receive a callback for the given
    /// correlation ID and entity.
    pub fn matching_webhooks(
        &self,
        entity_id: &EntityId,
        correlation_id: u64,
    ) -> Vec<&WebhookEntry> {
        self.webhooks
            .values()
            .filter(|e| e.entity_id == *entity_id)
            .filter(|e| match &e.filter_correlation_ids {
                Some(ids) => ids.contains(&correlation_id),
                None => true,
            })
            .collect()
    }

    /// Get all registered webhooks.
    pub fn list_all(&self) -> Vec<(&str, &WebhookEntry)> {
        self.webhooks
            .iter()
            .map(|(id, entry)| (id.as_str(), entry))
            .collect()
    }

    /// Count of registered webhooks.
    pub fn count(&self) -> usize {
        self.webhooks.len()
    }

    /// Deliver a webhook callback (fire-and-forget, logs errors).
    ///
    /// In production this would use `reqwest` to POST JSON to the URL.
    /// For now, we return the payload that would be sent.
    pub fn prepare_payload(
        &self,
        correlation_id: u64,
        result: &str,
        status: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "correlation_id": correlation_id.to_string(),
            "result": result,
            "status": status,
        })
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entity() -> EntityId {
        EntityId::new("app-1")
    }

    #[test]
    fn test_register_and_list() {
        let mut mgr = WebhookManager::new();
        let id = mgr.register(WebhookEntry {
            entity_id: test_entity(),
            url: "https://example.com/hook".into(),
            filter_correlation_ids: None,
        });
        assert_eq!(id, "wh-0");
        assert_eq!(mgr.count(), 1);

        let all = mgr.list_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].1.url, "https://example.com/hook");
    }

    #[test]
    fn test_unregister() {
        let mut mgr = WebhookManager::new();
        let id = mgr.register(WebhookEntry {
            entity_id: test_entity(),
            url: "https://example.com/hook".into(),
            filter_correlation_ids: None,
        });
        assert!(mgr.unregister(&id));
        assert_eq!(mgr.count(), 0);
        assert!(!mgr.unregister("nonexistent"));
    }

    #[test]
    fn test_unregister_entity() {
        let mut mgr = WebhookManager::new();
        let e1 = EntityId::new("app-1");
        let e2 = EntityId::new("app-2");

        mgr.register(WebhookEntry {
            entity_id: e1.clone(),
            url: "https://a.com".into(),
            filter_correlation_ids: None,
        });
        mgr.register(WebhookEntry {
            entity_id: e2.clone(),
            url: "https://b.com".into(),
            filter_correlation_ids: None,
        });
        mgr.register(WebhookEntry {
            entity_id: e1.clone(),
            url: "https://c.com".into(),
            filter_correlation_ids: None,
        });

        mgr.unregister_entity(&e1);
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_matching_webhooks_no_filter() {
        let mut mgr = WebhookManager::new();
        mgr.register(WebhookEntry {
            entity_id: test_entity(),
            url: "https://example.com".into(),
            filter_correlation_ids: None,
        });

        let matches = mgr.matching_webhooks(&test_entity(), 42);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_matching_webhooks_with_filter() {
        let mut mgr = WebhookManager::new();
        mgr.register(WebhookEntry {
            entity_id: test_entity(),
            url: "https://example.com".into(),
            filter_correlation_ids: Some(vec![1, 2, 3]),
        });

        let matches = mgr.matching_webhooks(&test_entity(), 2);
        assert_eq!(matches.len(), 1);

        let matches = mgr.matching_webhooks(&test_entity(), 99);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_matching_webhooks_wrong_entity() {
        let mut mgr = WebhookManager::new();
        mgr.register(WebhookEntry {
            entity_id: test_entity(),
            url: "https://example.com".into(),
            filter_correlation_ids: None,
        });

        let matches = mgr.matching_webhooks(&EntityId::new("other"), 42);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_prepare_payload() {
        let mgr = WebhookManager::new();
        let payload = mgr.prepare_payload(123, "⟦Σ∞⟧⟬I:✕⟭", "completed");
        assert_eq!(payload["correlation_id"], "123");
        assert_eq!(payload["result"], "⟦Σ∞⟧⟬I:✕⟭");
        assert_eq!(payload["status"], "completed");
    }
}
