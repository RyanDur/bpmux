# P2P Networking Design

Dev design doc — 2026-04-09. Based on battleship prior art, designer UX spec, and team architecture discussion.

## Problem

bpmux runs on one machine. When the team moves to a new machine, the lesson index, log, and session history stay behind. Without networking, knowledge is lost or requires file copying. bpmux needs P2P connections so two instances share understanding as equals.

## Constraints

- No startup flags — networking is built-in
- No central server — pure P2P
- Secure by default — encrypted connections
- Runtime connection — `/session` and `/join`, not config files
- Same experience as local — remote peers indistinguishable from local
- Token-efficient — no unnecessary overhead

## Connection Flow

Two-round WebRTC SDP exchange, encrypted with a shared passphrase:

```
Machine A: /session
  → generates WebRTC offer
  → encrypts SDP with passphrase (PBKDF2 → AES-GCM)
  → displays offer code + passphrase

Machine B: /join <offer-code>
  → decrypts offer SDP with passphrase
  → creates WebRTC answer
  → encrypts answer SDP
  → displays answer code

Machine A: /accept <answer-code>
  → decrypts answer SDP
  → WebRTC data channel established
```

The passphrase is shared out-of-band (verbally, text, etc). It authenticates and encrypts — no separate auth layer needed.

## Architecture

### Transport Trait

```rust
trait PeerTransport: Send + Sync {
    async fn send(&self, msg: &str) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<String, TransportError>;
    fn is_connected(&self) -> bool;
}
```

WebSocket and WebRTC data channels both implement this trait. The session broadcast doesn't care which transport a message arrived on.

### Session Changes

The Session struct gains a `peers` field — connected remote bpmux instances. Each peer has:
- A transport (WebRTC data channel)
- A name/identity
- A sync state (which messages have been relayed)

### Message Relay

When a message is broadcast locally, it's also relayed to all connected peers. When a message arrives from a peer, it's broadcast locally. Loop prevention: messages carry an origin ID — don't relay back to the origin.

### Lesson Sync

On peer connect:
1. Each peer sends its lesson index to the other
2. New lessons are merged (same topic + same role = keep the newer one)
3. After sync, both peers have the union of lessons

On new lesson saved:
1. The lesson is broadcast to all connected peers
2. Peers merge it into their local index

### Log Access

Each peer keeps its own log. Search queries are local — you search your own log. Cross-peer search (searching the other machine's log) is a future feature, not v1.

## Rust Crates

- `webrtc` (0.8.0) — pure Rust WebRTC: peer connections, data channels, ICE
- Standard library crypto for PBKDF2 + AES-GCM (or `ring` crate)
- `mdns-sd` for optional mDNS discovery on LAN

## Phased Delivery

### Phase 1: Direct connect (two-round code exchange)
- `/session` generates encrypted offer code
- `/join` + `/accept` complete the handshake
- Messages relay bidirectionally over data channel
- Lesson sync on connect

### Phase 2: LAN discovery (mDNS)
- Automates the SDP exchange on same network
- `/session` announces via mDNS
- `/join` discovers without a code
- Falls back to code exchange if mDNS fails

### Phase 3: Multi-peer
- More than two instances connected
- Message relay with loop prevention
- Lesson sync across the mesh

## Open Questions

1. Should the passphrase be human-readable ("blue-tiger-42") or a random string? Human-readable is easier to share verbally but has less entropy.
2. How long should codes be? Battleship codes are long base64url strings. Short codes trade security for usability.
3. Should peers share logs or just lessons? Sharing logs is expensive. Sharing lessons is cheap. Log search across peers is a future feature.
4. What happens when lessons conflict? Two peers have different lessons on the same topic from the same role. Most recent timestamp wins? Or both kept?
