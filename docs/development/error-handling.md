# Error Handling

## Error Hierarchy

Each major crate defines its own `thiserror`-derived error enum.
Cross-crate propagation uses explicit `From` impls or `.map_err()`.

### nexus-common — `NexusError` (foundation)

13 variants covering crypto, network, agent, task, identity, and
serialization failures. Used by all downstream crates.

```
NexusError
├── EncryptionError(String)
├── DecryptionError(String)
├── SerializationError(serde_json::Error)     ← From<serde_json::Error>
├── NetworkError(String)                      ← From<std::io::Error>
├── InvalidMessage(String)
├── AgentError(String)
├── TaskExecutionError(String)
├── ConfigurationError(String)
├── UnknownTechnique(String)
├── InvalidIdentity(String)
├── SignatureVerificationFailed
├── CryptoFailure(String)
└── BincodeError(String)
```

### nexus-infra — `InfraError`

10 variants for infrastructure operations. Does **not** implement
`From<NexusError>` — the two hierarchies are kept separate to avoid
blanket conversions that lose context.

```
InfraError
├── ConfigError(String)
├── CloudflareError(String)
├── CertificateError(String)
├── LetsEncryptError(String)
├── DomainError(String)
├── GrpcError(String)
├── NetworkError(reqwest::Error)              ← From<reqwest::Error>
├── TlsError(String)
├── IoError(String)                           ← From<std::io::Error>
└── BofError(String)
```

### nexus-a2a — module-specific errors

Each module defines its own error type to keep boundaries tight:

| Error | Module | Variants |
|---|---|---|
| `CardError` | `cards.rs` | Parse, Verify, Serialize |
| `TokenError` | `tokens.rs` | Expired, InvalidSignature, Malformed |
| `TlsError` | `tls.rs` | CertLoad, KeyLoad, KeyMismatch |
| `CapabilityError` | `capabilities.rs` | Denied, InvalidMatrix |
| `S3SinkError` | `audit_s3.rs` | Upload, Config, FeatureDisabled |
| `OtelError` | `otel.rs` | Init, FeatureDisabled |

### nexus-mesh — `DtnError`

Covers DTN queue I/O failures. Implements `From<std::io::Error>`.

## `From` Conversion Map

```
std::io::Error ──► NexusError::NetworkError
std::io::Error ──► InfraError::IoError
std::io::Error ──► DtnError::Io

serde_json::Error ──► NexusError::SerializationError
reqwest::Error ──► InfraError::NetworkError
```

No cross-crate `From` impls exist. This is intentional — it forces
callers to explicitly map errors at crate boundaries, preserving
context.

## Patterns

- **Library crates** (`nexus-common`, `nexus-a2a`, `nexus-mesh`):
  use `thiserror` enums. Return `Result<T, SpecificError>`.
- **Binary boundaries** (`nexus-server`, `nexus-agent`): use `anyhow`
  for top-level error propagation. Convert typed errors via `?`.
- **Cross-crate propagation**: use `.map_err()` to wrap the source
  error with context:
  ```rust
  nexus_a2a::tls::load_server_config()
      .map_err(|e| InfraError::TlsError(format!("A2A TLS: {e}")))?;
  ```
