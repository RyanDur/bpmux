# WebRTC Signaling Patterns for bpmux

*Researched April 2, 2026. Based on production patterns, battleship's implementation, and str0m ecosystem.*

---

## Minimal Signaling Server Design

bpmux needs the simplest possible signaling server: accept two WebSocket connections, match by session code, relay SDP/ICE messages between them.

**Architecture:** axum WebSocket handler with `HashMap<SessionCode, Session>`:
- Peer A connects with `CREATE` → session code generated, A's sender stored
- Peer B connects with `JOIN <code>` → session looked up, B's sender stored, relay active
- Messages forwarded without inspection
- WebSocket stays open after handshake for ICE restart signaling

**No database. No lobby. No broadcast.** In-memory session map only. Sessions exist while WebSockets are alive + short grace period.

## Session Code Format

**Crockford Base32:** `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (32 chars)
- Excludes I, L, O, U — avoids visual ambiguity (0/O, 1/I/L) and accidental profanity
- Case-insensitive decoding (`i` → `1`, `o` → `0`)
- Hyphens for readability

**Recommended:** 6 characters = 32^6 ≈ 1 billion combinations (30 bits entropy). Format: `XXX-XXX` (e.g., `7HK-M3P`).

**Entropy source:** `rand::rngs::OsRng` (CSPRNG). No counter or timestamp needed — codes are ephemeral.

**Expiry:** Single-use AND 10-minute TTL. Code consumed when peer B joins. Stale codes cleaned on TTL.

## Signaling Protocol

JSON messages with `type` discriminator, deserialized via `#[serde(tag = "type")]` enum:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum SignalingMessage {
    Create { name: String },
    Created { code: String },
    Join { code: String, name: String },
    Joined { peer_name: String },
    Offer { sdp: String },
    Answer { sdp: String },
    IceRestart { sdp: String },
    IceRestartAnswer { sdp: String },
    Error { message: String },
}
```

This mirrors battleship's `when(payload["type"])` dispatch but leverages Rust's enum + serde for type-safe deserialization.

## Signaling Security

**Three layers:**

1. **TLS on signaling channel (WSS)** — protects against passive eavesdropping. Required for hosted server, optional on localhost.

2. **Signaling server trust** — the server sees SDP in plaintext and could MITM by substituting DTLS fingerprints. For self-hosted: acceptable (you trust your own server). For hosted: a real concern.

3. **SAS verification (v1.x)** — after DTLS handshake, derive a short code from fingerprints. Both humans verify verbally. Catches active MITM even by a compromised server. Low-effort, high-value feature.

The manual mode (`--manual`) inherently avoids signaling server MITM — SDP is exchanged directly between humans out-of-band.

## Patterns Adopted from Battleship

| Pattern | Battleship | bpmux |
|---|---|---|
| WebSocket message dispatch | `when(payload["type"])` | `#[serde(tag = "type")]` enum |
| Peer-to-session mapping | Two `ConcurrentHashMap`s | `DashMap` or `RwLock<HashMap>` |
| ICE restart over persistent WS | `RELAY_ICE_RESTART` messages | Same pattern, same message types |
| Peer identity | Cookie-based `peerId` | Session code + peer name from first message |

## Patterns Simplified from Battleship

| Battleship Pattern | bpmux Simplification |
|---|---|
| Multi-peer lobby with broadcast | Strictly two-peer sessions, no lobby |
| JPA persistent storage | In-memory only, no database |
| Game state in signaling server | Signaling server is relay only, no application logic |
| Composite pair keys (`peerId1:peerId2`) | Session code is the key |

## str0m and Signaling

str0m is sans-I/O and does not prescribe signaling. The signaling server is completely independent of str0m. str0m lives in the bpmux node; the signaling server is a dumb WebSocket relay.

## Sources

- [matchbox_server (axum WebSocket signaling)](https://github.com/johanhelsing/matchbox)
- [str0m examples](https://github.com/algesten/str0m/tree/main/examples)
- [Battleship SignalingHandler.kt](/Users/ryandurling/IdeaProjects/battleship/applications/signaling-server/src/main/kotlin/com/battleship/backend/signaling/SignalingHandler.kt)
- [RFC 8827 — WebRTC Security Architecture](https://datatracker.ietf.org/doc/html/rfc8827)
