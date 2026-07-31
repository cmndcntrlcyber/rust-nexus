# shell-session — A2A skill framing (v1.0 protocol, used by v1.1+)

The `shell-session` skill rides on the A2A `SendStreamingMessage` bidi RPC.

## Wire RPC

```
service A2aService {
  rpc SendStreamingMessage(stream Message) returns (stream StreamResponse);
}
```

Each `Message` carries one or more `Part`s. v1.1 uses:

| `Part.part` variant | Purpose                                       |
|---------------------|-----------------------------------------------|
| `text`              | JSON-serialized `ShellControl` (control frame) |
| `file`              | Raw PTY bytes                                 |

## Control frames

```jsonc
// operator → agent (always first)
{"kind":"shell-open","cols":80,"rows":24}
// or with target / shell override:
{"kind":"shell-open","cols":80,"rows":24,"shell":"/bin/zsh",
 "target_agent_id":"ab12cd34…64-char-hex"}

// operator → agent
{"kind":"shell-resize","cols":120,"rows":40}

// agent → operator (last frame)
{"kind":"shell-exit","code":0}
```

### `shell-open` fields

- `cols` (u16, required)
- `rows` (u16, required)
- `shell` (string, optional) — override or `null` for OS-aware default
- `target_agent_id` (string, optional) — hex-encoded 32-byte peer id; the
  v1.1 C2 router resolves this via the registry-lister bridge

If the first inbound frame isn't a parseable `shell-open`, the server
closes the stream with `Status::InvalidArgument`.

## Byte frames

Raw `Part.file` chunks. UTF-8 sequences may be split across chunks;
consumers must concatenate before rendering.

## v1.1 routing (additive integration)

The operator console connects to the C2's A2A port (separate from the
overlay's NexusC2 port). The C2's `OperatorRouter` (`nexus_infra::a2a_router`)
looks up the target agent in an `AgentChannels` table, returning
`FailedPrecondition` if the target hasn't registered.

Visibility (`ListRegisteredAgents`) bridges to the overlay's existing
`AgentSession` registry via `nexus_infra::a2a_lister::RegistryLister`,
mapping each agent's UUID to a 32-byte peer-id via `BLAKE3(uuid.bytes)`.

## v1.2 additions

### `agent-register` control frame

The first inbound frame on an agent-mode bidi stream identifies the
agent so the C2's `AgentRegistrar` can populate `AgentChannels`:

```jsonc
{"kind":"agent-register","peer_id_hex":"ab12…64-char-hex",
 "os":"linux","version":"0.2.0","tag":"prod-host-1"}
```

`os` is one of `"linux" | "windows" | "macos" | "other"`. `version` and
`tag` are optional. The server distinguishes operator-mode vs agent-mode
streams on the first frame's `kind`.

### Signed `AgentCard`

`GetAgentCard` responses include `signature` (Ed25519, 64 bytes) and
`signer_peer_id` (Ed25519 verifying key, 32 bytes). Verification rules:

- Canonical encoding: JSON-encode `{description, name, skills, version}`
  with sorted keys, no whitespace, signature fields zeroed.
- `sign(canonical_bytes, signer_secret)` → 64-byte signature.
- v1.1 clients ignore unknown fields and treat the card as unsigned.

### mTLS (D-V1-E reversal)

A2A streams in v1.2 typically run over mTLS; certs are loaded from
`NEXUS_*_CERT` env vars. Operator authentication binds to the client
cert's CN/SAN for the capability matrix check.
