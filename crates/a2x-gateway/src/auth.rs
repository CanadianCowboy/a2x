// See plans/06-entity-gateway.md §6
// Authentication and authorization for the A2X gateway.

use std::collections::HashMap;

use crate::entity::EntityId;
use crate::error::GatewayError;

/// How an entity authenticates with the gateway.
#[derive(Clone, Debug)]
pub enum AuthMethod {
    /// API key in HTTP header (X-A2X-Key).
    ApiKey(String),
    /// Bearer token (JWT — validated externally).
    BearerToken(String),
    /// No auth required (local connections, e.g. Unix socket / stdio).
    Local,
}

/// What an entity is allowed to do.
#[derive(Clone, Debug)]
pub struct EntityPermissions {
    /// Entity ID these permissions belong to.
    pub entity_id: EntityId,
    /// Maximum instruction count per program.
    pub max_instructions: u64,
    /// Can this entity probe agent state?
    pub can_probe: bool,
    /// Can this entity access external network?
    pub can_network: bool,
    /// Per-minute rate limit (0 = unlimited).
    pub rate_limit: u32,
}

impl Default for EntityPermissions {
    fn default() -> Self {
        EntityPermissions {
            entity_id: EntityId::new("default"),
            max_instructions: 10_000,
            can_probe: false,
            can_network: false,
            rate_limit: 0,
        }
    }
}

/// Trait for authenticating entities.
pub trait AuthProvider: Send + Sync {
    /// Validate an auth method and return the entity ID if valid.
    fn authenticate(&self, method: &AuthMethod) -> Result<EntityId, GatewayError>;

    /// Get permissions for an entity.
    fn permissions(&self, entity_id: &EntityId) -> Option<EntityPermissions>;
}

/// In-memory auth provider — simple API key store.
pub struct InMemoryAuthProvider {
    /// API key → entity ID mapping.
    keys: HashMap<String, EntityId>,
    /// Entity ID → permissions mapping.
    permissions: HashMap<EntityId, EntityPermissions>,
}

impl InMemoryAuthProvider {
    pub fn new() -> Self {
        InMemoryAuthProvider {
            keys: HashMap::new(),
            permissions: HashMap::new(),
        }
    }

    /// Register an API key for an entity.
    pub fn register_key(&mut self, key: String, entity_id: EntityId) {
        self.keys.insert(key, entity_id);
    }

    /// Set permissions for an entity.
    pub fn set_permissions(&mut self, perms: EntityPermissions) {
        self.permissions.insert(perms.entity_id.clone(), perms);
    }
}

impl Default for InMemoryAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthProvider for InMemoryAuthProvider {
    fn authenticate(&self, method: &AuthMethod) -> Result<EntityId, GatewayError> {
        match method {
            AuthMethod::ApiKey(key) => self
                .keys
                .get(key)
                .cloned()
                .ok_or_else(|| GatewayError::AuthFailed("invalid API key".into())),
            AuthMethod::BearerToken(_token) => {
                // Phase 6: stub JWT validation — accept any non-empty token
                // TODO: real JWT validation in a future phase
                Ok(EntityId::new("jwt-entity"))
            }
            AuthMethod::Local => Ok(EntityId::new("local")),
        }
    }

    fn permissions(&self, entity_id: &EntityId) -> Option<EntityPermissions> {
        self.permissions.get(entity_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_auth() {
        let mut auth = InMemoryAuthProvider::new();
        let eid = EntityId::new("app-1");
        auth.register_key("sk-test-123".into(), eid.clone());

        let result = auth.authenticate(&AuthMethod::ApiKey("sk-test-123".into()));
        assert_eq!(result.unwrap(), eid);

        let result = auth.authenticate(&AuthMethod::ApiKey("bad-key".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_local_auth() {
        let auth = InMemoryAuthProvider::new();
        let result = auth.authenticate(&AuthMethod::Local);
        assert_eq!(result.unwrap(), EntityId::new("local"));
    }

    #[test]
    fn test_permissions() {
        let mut auth = InMemoryAuthProvider::new();
        let eid = EntityId::new("app-1");
        auth.set_permissions(EntityPermissions {
            entity_id: eid.clone(),
            max_instructions: 500,
            can_probe: true,
            can_network: false,
            rate_limit: 100,
        });

        let perms = auth.permissions(&eid).unwrap();
        assert_eq!(perms.max_instructions, 500);
        assert!(perms.can_probe);
        assert!(!perms.can_network);
        assert_eq!(perms.rate_limit, 100);
    }

    #[test]
    fn test_jwt_auth_stub() {
        let auth = InMemoryAuthProvider::new();
        let result = auth.authenticate(&AuthMethod::BearerToken("eyJhbG...".into()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_permissions() {
        let perms = EntityPermissions::default();
        assert_eq!(perms.max_instructions, 10_000);
        assert!(!perms.can_probe);
        assert_eq!(perms.rate_limit, 0);
    }
}
