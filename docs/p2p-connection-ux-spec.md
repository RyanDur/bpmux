# P2P Connection UX Spec

*Designer spec — 2026-04-09. Based on battleship prior art study and WebRTC constraints.*

## Connection Flow (Two-Round SDP Exchange)

WebRTC requires an offer-answer exchange. The UX wraps this in three commands across two machines.

### Machine A (host — has the session with lessons)

```
ryan > /session
Session created. Share this code with the other machine:
  [offer-code]
Waiting for peer to respond...
```

The offer code is the encrypted SDP offer (AES-256-GCM, encrypted with a passphrase derived from the session). Long string — designed to be copy-pasted, not spoken.

### Machine B (joining — fresh bpmux instance)

```
ryan > /join [offer-code]
Connected to peer. Share this answer code back:
  [answer-code]
```

Machine B decodes the offer, establishes its side of the connection, and generates the answer code.

### Machine A (completing the connection)

```
Peer responded. Connection established.
● machine-B connected
Syncing lessons... 24 lessons received.
```

If mDNS is available on the same network, the answer round can be automated — Machine A discovers Machine B's answer without manual code exchange. The UX becomes:

```
ryan > /session
Session created. Peer "machine-B" found on network. Connected automatically.
● machine-B connected
```

## What the User Sees After Connection

### Status Bar
```
[all] ● designer ● dev ● pm  ● machine-B (peer)   ?=help
```

Remote peers show with a "(peer)" label to distinguish from local agents.

### Sidebar
```
channels
  broadcast
  designer
  dev
  pm
  ── peers ──
  machine-B
```

A "peers" section in the sidebar shows connected machines. Click to see the peer's lessons.

### System Messages
```
[system] machine-B connected as peer
[system] 24 lessons synced from machine-B
```

Connection events appear as system messages in the chat.

## Lesson Sync

On connection, peers exchange lesson indexes bidirectionally:
- Machine A shares its 24 lessons with Machine B
- Machine B shares its lessons with Machine A (if any)
- Each machine merges the other's lessons into its own index, tagged with source_peer
- `/lessons` shows all lessons with source attribution

## Disconnect

```
[system] machine-B disconnected
```

- Remote peer's dot disappears from status bar
- Remote peer's entry removed from sidebar "peers" section
- Lessons from that peer remain in the index (they don't disappear on disconnect)
- Local session continues uninterrupted

## Reconnect

If the peer reconnects, lessons re-sync to pick up anything new since the disconnect. No data loss.

## Security

- SDP exchange is encrypted with a passphrase (AES-256-GCM, PBKDF2 key derivation)
- The passphrase is derived from the session token
- WebRTC data channels use DTLS encryption by default
- No authentication bypass — the encrypted offer code IS the authentication

## Constraints

- Two-round exchange is inherent to WebRTC (offer + answer)
- mDNS discovery only works on the same LAN
- Relay servers needed for WAN connections (future scope)
- The codes are long (encoded SDP) — not human-speakable

## Open Questions

1. Can we shorten the codes? The battleship codes are long base64 strings. A short human-readable phrase that maps to the full SDP via a relay would be better UX but requires infrastructure.
2. Should peers auto-reconnect on network changes? WebRTC ICE restart supports this.
3. How many peers can connect simultaneously? WebRTC supports multiple peer connections but each adds overhead.
