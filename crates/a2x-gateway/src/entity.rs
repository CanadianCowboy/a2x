// See plans/06-entity-gateway.md §3
// Entity trait and types for the A2X gateway.

pub use a2x_core::Capability;

/// Unique identifier for an external entity connected to the gateway.
///
/// Distinct from `AgentId` — entities are external systems that connect
/// through the gateway, while agents are native A2X runtimes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(id: impl Into<String>) -> Self {
        EntityId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EntityId {
    fn from(s: &str) -> Self {
        EntityId(s.to_string())
    }
}

impl From<String> for EntityId {
    fn from(s: String) -> Self {
        EntityId(s)
    }
}

/// Re-export EntityType from core for convenience.
pub use a2x_core::EntityType;

/// Information about a registered entity (for listing/discovery).
#[derive(Clone, Debug)]
pub struct EntityInfo {
    pub id: EntityId,
    pub entity_type: EntityType,
    pub display_name: String,
    pub capabilities: Vec<Capability>,
    pub connected_at: std::time::Instant,
}

impl EntityInfo {
    pub fn new(
        id: EntityId,
        entity_type: EntityType,
        display_name: impl Into<String>,
        capabilities: Vec<Capability>,
    ) -> Self {
        EntityInfo {
            id,
            entity_type,
            display_name: display_name.into(),
            capabilities,
            connected_at: std::time::Instant::now(),
        }
    }
}

/// Represents an external entity connected to the A2X ecosystem.
///
/// Entities do NOT run a CCS VM internally. Instead, they connect through
/// the gateway, which translates between the entity's native protocol and
/// A2X bus messages.
///
/// See plans/06-entity-gateway.md §3 for the full trait specification.
pub trait Entity: Send + Sync {
    /// Unique entity ID.
    fn entity_id(&self) -> EntityId;

    /// Entity type tag.
    fn entity_type(&self) -> EntityType;

    /// Human-readable name (for display/probe).
    fn display_name(&self) -> String;

    /// Check if the entity is still connected.
    fn is_alive(&self) -> bool;

    /// Entity capabilities (what this entity can do).
    fn capabilities(&self) -> Vec<Capability>;
}

/// A simple entity backed by metadata (no transport).
///
/// Useful for testing and for entities registered statically via config.
pub struct SimpleEntity {
    pub info: EntityInfo,
    pub alive: bool,
}

impl SimpleEntity {
    pub fn new(info: EntityInfo) -> Self {
        SimpleEntity { info, alive: true }
    }
}

impl Entity for SimpleEntity {
    fn entity_id(&self) -> EntityId {
        self.info.id.clone()
    }

    fn entity_type(&self) -> EntityType {
        self.info.entity_type
    }

    fn display_name(&self) -> String {
        self.info.display_name.clone()
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.info.capabilities.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_creation() {
        let id = EntityId::new("entity-1");
        assert_eq!(id.as_str(), "entity-1");
        assert_eq!(format!("{}", id), "entity-1");
    }

    #[test]
    fn test_entity_id_from_str() {
        let id: EntityId = "test".into();
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn test_entity_info_creation() {
        let info = EntityInfo::new(
            EntityId::new("e-1"),
            EntityType::HumanCli,
            "Test Entity",
            vec![Capability::Execute],
        );
        assert_eq!(info.id.as_str(), "e-1");
        assert_eq!(info.display_name, "Test Entity");
    }

    #[test]
    fn test_simple_entity_trait() {
        let info = EntityInfo::new(
            EntityId::new("e-1"),
            EntityType::Application,
            "App",
            vec![Capability::Execute, Capability::Network],
        );
        let entity = SimpleEntity::new(info);
        assert_eq!(entity.entity_id(), EntityId::new("e-1"));
        assert_eq!(entity.entity_type(), EntityType::Application);
        assert!(entity.is_alive());
        assert_eq!(entity.capabilities().len(), 2);
    }
}
