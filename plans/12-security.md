# A2X Security Model — Authentication, Encryption & Access Control

> **How agents, entities, and users authenticate to each other; how the bus is secured; how API keys are stored and rotated.**

---

## 1. Overview

A2X has three security domains:

| Domain | What's Protected | Threats |
|--------|-----------------|---------|
| **Agent-to-Agent** | Bus messages, agent identity | Impersonation, message tampering, eavesdropping |
| **Entity-to-Gateway** | Gateway connections, API access | Unauthorized access, injection, replay |
| **Host System** | CLI agent execution, filesystem | Malicious programs, privilege escalation |

---

## 2. Agent-to-Agent Security

### Agent Identity

Each agent has an `AgentId` and a cryptographic key pair:

```rust
/// Agent identity — created once per agent instance.
pub struct AgentIdentity {
    /// Unique agent ID.
    pub id: AgentId,
    /// Ed25519 key pair for signing.
    pub signing_key: ed25519_dalek::SigningKey,
    /// Verifying key (shared on the bus during announcement).
    pub verifying_key: ed25519_dalek::VerifyingKey,
}

impl AgentIdentity {
    /// Generate a new identity.
    pub fn generate(id: AgentId) -> Self { /* ... */ }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Verify a signature from another agent.
    pub fn verify(verifying_key: &VerifyingKey, message: &[u8], sig: &Signature) -> bool {
        verifying_key.verify(message, sig).is_ok()
    }
}
```

### Bus Message Signing

Every `WireMessage` can be signed:

```rust
pub struct SignedWireMessage {
    /// The original wire message (serialized bytes).
    pub payload: Vec<u8>,
    /// Sender's Ed25519 signature over payload.
    pub signature: [u8; 64],
    /// Sender's verifying key (sent with first message only).
    pub verifying_key: Option<[u8; 32]>,
}
```

**First contact protocol:**
1. Agent A connects to bus, sends `Announce` with its `verifying_key`
2. Bus stores `AgentId → VerifyingKey` mapping
3. All subsequent messages from A include a signature
4. Bus verifies signatures before routing (if `bus.verify_signatures = true`)

### Bus Encryption (Optional)

For transit over untrusted networks:

```rust
pub enum BusEncryption {
    /// No encryption (local development only).
    None,
    /// TLS between agents (recommended for LAN).
    Tls {
        cert_path: PathBuf,
        key_path: PathBuf,
        ca_path: Option<PathBuf>,
    },
    /// NaCl box (curve25519 + xsalsa20-poly1305) for p2p encrypted channels.
    Noise(NoiseConfig),
}
```

### Agent Trust Model

```rust
pub enum AgentTrust {
    /// Trust all agents on the bus (local network).
    Open,
    /// Only trust agents whose keys are in the allowlist.
    Allowlist(HashMap<AgentId, VerifyingKey>),
    /// Only trust agents signed by a trusted CA.
    CertificateAuthority(Vec<Vec<u8>>), // CA certs
}
```

---

## 3. Entity-to-Gateway Security

### Authentication Methods

```rust
pub enum EntityAuthMethod {
    /// API key in `X-A2X-Key` header.
    ApiKey { key: String },
    /// Bearer token (JWT or opaque).
    BearerToken { token: String },
    /// TLS client certificate.
    ClientCert { cert: Vec<u8> },
    /// No auth (Unix socket / localhost only).
    Local,
}
```

### API Key Storage

API keys are stored hashed, never in plaintext:

```rust
/// Stored API key entry (in config or database).
pub struct StoredApiKey {
    /// Human-readable label (e.g., "josh's dev machine").
    pub label: String,
    /// Blake3 hash of the key (not the key itself).
    pub key_hash: [u8; 32],
    /// Salt for the hash.
    pub salt: [u8; 16],
    /// Permissions granted to this key.
    pub permissions: EntityPermissions,
    /// When this key was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this key expires (None = never).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this key is revoked.
    pub revoked: bool,
}
```

```toml
# ~/.a2x/gateway.toml — API key section
[auth]
mode = "api_key"
hash_algorithm = "blake3"  # never store raw keys

[auth.api_keys]
# Format: label = "blake3_hash:salt"
"josh-dev" = "a1b2c3...:s1a2lt..."
"ci-pipeline" = "d4e5f6...:s3a4lt..."
```

### JWT Authentication

```rust
pub struct JwtConfig {
    /// HMAC secret or Ed25519 private key for signing.
    pub signing_key: JwtSigningKey,
    /// Issuer claim.
    pub issuer: String,
    /// Token expiration (default: 1 hour).
    pub expiration: Duration,
    /// Optional: JWKS endpoint for key rotation.
    pub jwks_url: Option<String>,
}
```

---

## 4. Entity Permissions

```rust
pub struct EntityPermissions {
    /// Entity ID this permission belongs to.
    pub entity_id: EntityId,
    /// Maximum instructions per program.
    pub max_instructions: u64,
    /// Maximum memory allocation (bytes).
    pub max_memory_bytes: u64,
    /// Allowed opcodes (None = all allowed).
    pub allowed_opcodes: Option<Vec<Opcode>>,
    /// Allowed addressing modes.
    pub allowed_modes: Vec<AddressingMode>,
    /// Can probe agent state?
    pub can_probe: bool,
    /// Can access external network?
    pub can_network: bool,
    /// Can execute shell commands?
    pub can_exec: bool,
    /// Can read/write files?
    pub can_fs: FileSystemPermission,
    /// Per-minute rate limit.
    pub rate_limit: u32,
    /// Admin: can reconfigure agents/gateway.
    pub is_admin: bool,
}
```

**Permission hierarchy:**

```
Admin
  │
  ├── Developer
  │     ├── can_probe: true
  │     ├── can_exec: true (sandboxed)
  │     └── is_admin: false
  │
  ├── Service
  │     ├── max_instructions: 1000
  │     ├── can_network: true
  │     └── can_exec: false
  │
  └── ReadOnly
        ├── max_instructions: 100
        ├── can_probe: true (read-only)
        └── can_exec: false
```

---

## 5. Host System Security

### CLI Agent Sandboxing

The CLI agent is the highest-risk component. It executes shell commands on the host:

```rust
pub struct CliAgentSecurity {
    /// Command allowlist (glob patterns).
    pub allowed_commands: Vec<GlobPattern>,
    /// Command denylist (always rejected, checked first).
    pub forbidden_commands: Vec<GlobPattern>,
    /// Maximum execution time per command.
    pub max_execution_time: Duration,
    /// Maximum concurrent processes.
    pub max_concurrent_processes: usize,
    /// Sandbox mode.
    pub sandbox_mode: CliSandboxMode,
    /// Resource limits (ulimit equivalents).
    pub resource_limits: ResourceLimits,
}

pub enum CliSandboxMode {
    /// No sandboxing (dev only, requires `is_admin`).
    None,
    /// Filter commands against allowlist/denylist.
    CommandFilter,
    /// Run in a Linux container (nsjail / bubblewrap).
    Container { image: String, read_only_root: bool },
    /// Run in a micro-VM (firecracker, future).
    MicroVm { kernel: String, rootfs: String },
}

pub struct ResourceLimits {
    pub max_cpu_time: Duration,
    pub max_memory_bytes: u64,
    pub max_file_size: u64,
    pub max_open_files: u64,
    pub max_processes: u64,
}
```

### Rate Limiting

```rust
/// Token bucket rate limiter.
pub struct RateLimiter {
    max_per_minute: u32,
    tokens: AtomicU32,
    last_refill: AtomicI64, // Unix timestamp
}
```

Applied at three levels:
1. **Per-entity** — entity can't exceed its rate limit
2. **Per-agent** — agent can't exceed its processing capacity
3. **Global** — bus has a maximum throughput (configurable)

---

## 6. Key Rotation & Secrets Management

### Agent Key Rotation

```rust
pub enum KeyRotationPolicy {
    /// Never rotate (static keys).
    Never,
    /// Rotate every N days.
    TimeBased { interval_days: u32 },
    /// Rotate after N messages signed.
    UsageBased { max_signatures: u64 },
}
```

### Where Keys Live

| Secret | Storage | Format |
|--------|---------|--------|
| Agent signing key | `~/.a2x/keys/<agent-id>.key` | ed25519 binary (chmod 600) |
| Gateway TLS cert | `~/.a2x/gateway/cert.pem` | PEM |
| Gateway TLS key | `~/.a2x/gateway/key.pem` | PEM (chmod 600) |
| API key hashes | `~/.a2x/gateway.toml` | Blake3 hash:salt |
| Entity JWTs | Generated at runtime | In-memory only |

---

## 7. Audit Logging

All security-relevant events are logged:

```rust
pub enum SecurityEvent {
    /// Agent joined the bus.
    AgentJoined { id: AgentId, addr: SocketAddr },
    /// Agent disconnected.
    AgentLeft { id: AgentId },
    /// Entity authenticated.
    EntityAuthenticated { id: EntityId, method: AuthMethod },
    /// Authentication failure.
    AuthenticationFailure { ip: SocketAddr, reason: String },
    /// Permission denied.
    PermissionDenied { entity_id: EntityId, action: String },
    /// Safety violation.
    SafetyViolation { agent_id: AgentId, instruction: String, reason: String },
    /// Configuration change.
    ConfigChange { user: String, changes: Vec<String> },
    /// Key rotation.
    KeyRotated { agent_id: AgentId },
    /// Rate limit exceeded.
    RateLimited { entity_id: EntityId, count: u32 },
}
```

---

## 8. Security Checklist (Pre-Production)

- [ ] Agent Ed25519 key pair generation
- [ ] Bus message signing + verification
- [ ] Gateway API key authentication
- [ ] JWT token authentication option
- [ ] Entity permission model (read/write/exec/admin)
- [ ] CLI agent command filtering + sandbox mode
- [ ] Rate limiting (entity + global)
- [ ] Resource limits on CLI agent
- [ ] Audit logging for all security events
- [ ] Key rotation mechanism
- [ ] TLS support for bus transport
- [ ] TLS support for gateway HTTP/WS
- [ ] Secure key storage (file permissions)
- [ ] `forbidden_commands` denylist (rm -rf, sudo, etc.)
- [ ] Graceful handling of auth failures (no info leakage)

---

*This sub-plan maps to phases 1–6 of the implementation roadmap (incremental security hardening).*
