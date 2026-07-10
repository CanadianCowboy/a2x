# Security Policy

## Supported Versions

Security vulnerabilities are taken seriously. The following versions receive security patches:

| Version | Status | Support |
|---------|--------|:-------:|
| v0.9.x (alpha) | Current | ✅ Security patches |
| < v0.9.0 | Unsupported | ❌ Upgrade required |

As an alpha-stage project, security issues are addressed on a best-effort basis. Critical vulnerabilities will receive immediate attention regardless of version status.

---

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

### Responsible Disclosure Process

1. **Contact:** Send an encrypted report to the maintainers via GitHub's [private vulnerability reporting](https://github.com/CanadianCowboy/a2x/security/advisories/new) feature.

2. **Include in your report:**
   - A clear description of the vulnerability
   - Steps to reproduce (minimal proof-of-concept if possible)
   - Affected crate(s) and version(s)
   - Potential impact and severity assessment
   - Any suggested mitigations

3. **What to expect:**
   - **Acknowledgment:** Within 72 hours
   - **Assessment:** Within 5 business days
   - **Fix timeline:** Depends on severity. Critical: 48 hours. High: 1 week. Medium: next release. Low: tracked in backlog.
   - **Disclosure:** Coordinated with you. We will not disclose before a fix is available.

### Severity Classification

| Severity | Examples |
|----------|----------|
| **Critical** | Remote code execution, auth bypass, data exfiltration, token leakage |
| **High** | Privilege escalation, sandbox escape, denial of service, unsafe code violations |
| **Medium** | Information disclosure, race conditions, unsafe deserialization, resource exhaustion |
| **Low** | Best practice violations, missing hardening, theoretical attack vectors |

---

## Security Model

A2X has multiple security boundaries. Understand them before reporting:

### Trust Boundaries

| Boundary | Risk Surface | Mitigation |
|----------|-------------|------------|
| **Gateway ↔ Network** | HTTP, WebSocket, TCP listeners | TLS, API key auth, rate limiting |
| **Bus ↔ Agents** | Message routing, agent discovery | Agent identity verification, message validation |
| **CCS VM ↔ Host** | Agent execution, CLI commands | Sandbox mode, capability allowlist, max instruction limits |
| **Omega Compiler ↔ Input** | Malformed program compilation | Parser fuzzing, input validation, safe deserialization |
| **Work Reports ↔ Filesystem** | Arbitrary file writes | Path canonicalization, restricted write directories |

### Security Features

- **API Key authentication** — all gateway endpoints require `X-A2X-Key` header (when configured)
- **Rate limiting** — token bucket per entity, configurable thresholds
- **CLI sandboxing** — allowed command prefix filtering, execution timeouts, retry limits
- **VM safety constraints** — max instructions, max memory, capability bits per instruction
- **TLS transport** — available for bus communication between agents
- **Agent identity** — Ed25519 signing pipeline for agent authentication

### What A2X Does NOT Protect Against (Yet)

- Malicious agents on the same bus (trusted agent model — future: agent attestation)
- Side-channel attacks on CCS VM execution (future: constant-time VM)
- Supply chain attacks on dependencies (mitigation: vendored deps in future)
- Physical access to the host machine

---

## Security Best Practices for Users

### Running A2X in Production

```bash
# Always set an API key
export A2X_API_KEY="$(openssl rand -hex 32)"

# Use TLS for network listeners
# Configure TLS certificates in ~/.a2x/config.toml

# Enable sandbox mode for CLI agents
# Set safety.level = "sandboxed" in agent config

# Run behind a reverse proxy for additional hardening
# nginx, Caddy, or cloud load balancer with WAF
```

### Dependency Auditing

```bash
# Check for known vulnerabilities in dependencies
cargo audit

# Review dependency licenses for AGPL-3.0 compatibility
cargo deny check licenses
```

---

## Hall of Fame

Contributors who have responsibly disclosed security vulnerabilities will be acknowledged here (with permission).

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong><br>
  <em>Precision. Clarity. Operator-Grade.</em>
</p>
