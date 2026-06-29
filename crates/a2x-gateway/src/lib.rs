// a2x-gateway — Entity gateway, protocol listeners, auth, entity registry
// See plans/06-entity-gateway.md and PLAN.md §30

pub mod auth;
pub mod config;
pub mod entity;
pub mod error;
pub mod gateway;
pub mod listeners;
pub mod webhook;

// Re-exports
pub use auth::{AuthMethod, AuthProvider, EntityPermissions, InMemoryAuthProvider};
pub use config::GatewayConfig;
pub use entity::{Capability, Entity, EntityId, EntityInfo, EntityType};
pub use error::GatewayError;
pub use gateway::Gateway;
pub use listeners::{ProtocolListener, ProtocolListenerType};
pub use webhook::{WebhookEntry, WebhookManager};
