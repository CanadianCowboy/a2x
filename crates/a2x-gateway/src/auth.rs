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
            AuthMethod::BearerToken(token) => {
                // Validate JWT format: three base64url-encoded segments
                // separated by dots. This is a structural check, not
                // cryptographic verification (which requires key material).
                let segments: Vec<&str> = token.split('.').collect();
                if segments.len() != 3 {
                    return Err(GatewayError::AuthFailed(
                        "malformed JWT: expected 3 segments".into(),
                    ));
                }

                // Decode the payload (second segment) to extract claims.
                let payload_bytes = decode_base64url(segments[1]).map_err(|_| {
                    GatewayError::AuthFailed("malformed JWT: invalid base64 in payload".into())
                })?;

                // Parse as JSON and extract the 'sub' claim as entity ID.
                let claims: serde_json::Value =
                    serde_json::from_slice(&payload_bytes).map_err(|_| {
                        GatewayError::AuthFailed("malformed JWT: invalid JSON payload".into())
                    })?;

                let entity_id = claims
                    .get("sub")
                    .and_then(|v| v.as_str())
                    .unwrap_or("jwt-entity");

                // Check expiration if present.
                if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    if exp < now {
                        return Err(GatewayError::AuthFailed("JWT token expired".into()));
                    }
                }

                Ok(EntityId::new(entity_id))
            }
            AuthMethod::Local => Ok(EntityId::new("local")),
        }
    }

    fn permissions(&self, entity_id: &EntityId) -> Option<EntityPermissions> {
        self.permissions.get(entity_id).cloned()
    }
}

/// Minimal base64url decoder — avoids external crate dependency.
///
/// Converts URL-safe base64 (using '-' and '_') to standard base64,
/// adds padding, then decodes using a simple lookup table.
fn decode_base64url(input: &str) -> Result<Vec<u8>, String> {
    // Convert URL-safe chars to standard base64 and add padding.
    let std_b64: String = input
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    let padding = (4 - std_b64.len() % 4) % 4;
    let padded = format!("{}{}", std_b64, "=".repeat(padding));

    // Build lookup table for standard base64.
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [0u8; 128];
    for (i, &c) in alphabet.iter().enumerate() {
        lookup[c as usize] = i as u8;
    }
    lookup[b'=' as usize] = 0; // padding char maps to 0

    let bytes = padded.as_bytes();
    let mut output = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            break;
        }
        let a = lookup
            .get(bytes.get(i).copied().unwrap_or(0) as usize)
            .copied()
            .unwrap_or(0);
        let b = lookup
            .get(bytes.get(i + 1).copied().unwrap_or(0) as usize)
            .copied()
            .unwrap_or(0);
        let c = lookup
            .get(bytes.get(i + 2).copied().unwrap_or(b'=') as usize)
            .copied()
            .unwrap_or(0);
        let d = lookup
            .get(bytes.get(i + 3).copied().unwrap_or(b'=') as usize)
            .copied()
            .unwrap_or(0);

        let triple = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);
        output.push(((triple >> 16) & 0xFF) as u8);
        if bytes[i + 2] != b'=' {
            output.push(((triple >> 8) & 0xFF) as u8);
        }
        if bytes[i + 3] != b'=' {
            output.push((triple & 0xFF) as u8);
        }
        i += 4;
    }
    Ok(output)
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
    fn test_jwt_auth_structural_only() {
        let auth = InMemoryAuthProvider::new();
        // Bare token without dots — should fail structural validation.
        let result = auth.authenticate(&AuthMethod::BearerToken("not-a-jwt".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_auth_valid_format() {
        let auth = InMemoryAuthProvider::new();
        // header.payload.signature with valid base64url payload: {"sub":"app-1"}
        // base64url("{\"sub\":\"app-1\"}") = "eyJzdWIiOiJhcHAtMSJ9"
        let token = "eyJhbGciOiJub25lIn0.eyJzdWIiOiJhcHAtMSJ9.dummy";
        let result = auth.authenticate(&AuthMethod::BearerToken(token.into()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "app-1");
    }

    #[test]
    fn test_jwt_auth_expired() {
        let auth = InMemoryAuthProvider::new();
        // Payload with exp in the past: {"sub":"app-1","exp":1}
        // base64url("{\"sub\":\"app-1\",\"exp\":1}") = "eyJzdWIiOiJhcHAtMSIsImV4cCI6MX0"
        let token = "eyJhbGciOiJub25lIn0.eyJzdWIiOiJhcHAtMSIsImV4cCI6MX0.dummy";
        let result = auth.authenticate(&AuthMethod::BearerToken(token.into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_base64url_basic() {
        // "hello" in base64url = "aGVsbG8"
        let result = decode_base64url("aGVsbG8").unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_decode_base64url_with_padding() {
        // "ab" in base64url = "YWI" (no padding in URL-safe)
        let result = decode_base64url("YWI").unwrap();
        assert_eq!(result, b"ab");
    }

    #[test]
    fn test_default_permissions() {
        let perms = EntityPermissions::default();
        assert_eq!(perms.max_instructions, 10_000);
        assert!(!perms.can_probe);
        assert_eq!(perms.rate_limit, 0);
    }
}
