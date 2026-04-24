# #451 internal board abstraction — current vs proposed

Companion to `08-internal-board-abstraction.md`. Mermaid diagrams
rendering the same architecture the spec describes in prose.

## Current state (pre-#451)

```mermaid
flowchart TB
  subgraph agents[Agents + Owner]
    A[MCP clients]
  end

  subgraph bridge[bpmux bridge]
    H[MCP tool handlers<br/>bridge/tools/*.rs]
    T[StoryTracker trait<br/>1:1 with GH operations]
    O[OctocrabStoryTracker]
    G[Direct git subprocess<br/>view_story_commits<br/>view_commit_diff]
  end

  subgraph source[External source]
    GH[(GitHub Issues)]
    LG[(Local git repo)]
  end

  A --> H
  H --> T
  H -.->|shells out| G
  T --> O
  O <-->|every MCP call<br/>round-trips here| GH
  G <-.->|shells out<br/>per call| LG

  style T fill:#fdd
  style GH fill:#fdd
  style H fill:#ffe
```

**Problems visible in this shape**:

- Every MCP call round-trips to GH (or shells git) — latency per call + coupling + rate-limit exposure.
- Two separate direct-to-source paths (tracker + git-subprocess) each with their own failure modes: #411 / #412 git-subprocess CWD, #450 label-stale from label-order-priority state-derivation.
- `StoryTracker` trait leaks GH 1:1 into domain names.
- No canonical internal state. Labels drive state — the source of #450.
- GH-down = bridge-effectively-down.

## Proposed state (post-#451, with #452 peer-sync layered in)

```mermaid
flowchart TB
  subgraph agents[Agents + Owner]
    A[MCP clients]
  end

  subgraph bridge[bpmux bridge]
    H[MCP tool handlers]
    BS[BoardStore<br/>canonical in-memory state<br/>Story + StoryState enum<br/>BoardEvent log]
    SO[SyncOrchestrator<br/>sync-in poll<br/>sync-out drain<br/>gap-fill]
    SA[SourceAdapter trait]
    PS[PeerSync trait<br/>#452]
  end

  subgraph adapters[Adapters]
    GA[GitHubSourceAdapter]
    IA[InMemorySourceAdapter<br/>tests]
    WP[WebRtcPeerSync<br/>#452]
  end

  subgraph external[External]
    GH[(GitHub)]
    PEERS[(Other bridge peers<br/>#452)]
  end

  A --> H
  H -->|reads canonical state| BS
  H -->|mutations as BoardEvent| SO
  SO -->|apply| BS
  SO <-->|ingress/egress| SA
  SA --> GA
  SA --> IA
  GA <-->|poll + push<br/>async| GH
  SO <-->|ingress/egress| PS
  PS --> WP
  WP <-.->|WebRTC<br/>#452 weekend-scope| PEERS

  style BS fill:#cfc
  style SO fill:#cfc
  style SA fill:#cfc
  style PS fill:#efe
  style H fill:#cfc
```

**What this shape wins**:

- MCP handlers only touch `BoardStore` + `SyncOrchestrator` — zero direct source coupling.
- Canonical in-memory state; reads are instant, writes are `BoardEvent`.
- Source + peers are symmetric ingresses of `BoardEvent` into `BoardStore` (hexagonal orthogonality — `SourceAdapter` and `PeerSync` are peers, not layered).
- GH-down doesn't break bridge; `SyncOrchestrator` retries async; `BoardStore` serves last-known-good.
- Peer-sync (#452, weekend-scope) adds multi-peer without reshaping any #451 code.
- Retires #450 (label-stale class) — typed `StoryState` enum replaces label-order-priority derivation.
- Retires tracker's direct octocrab seams — eliminates #411/#412/#447 class.

Green nodes = new in #451. Light-green node (`PeerSync`) = layered in #452.

## Current migration position

- **Phases 1–3** shipped: domain + `BoardStore` + `SourceAdapter` trait + `InMemorySourceAdapter` + `GitHubSourceAdapter` + `SyncOrchestrator`.
- **Phase 4a** shipped: `BridgeServer` holds `Arc<BoardStore>` alongside tracker.
- **Phase 4b** shipped: `run_bridge` seeds `BoardStore` from tracker at startup; first handler (`list_ready`) reads from board.
- **Phase 4c–4z** pending: one-at-a-time handler migrations from `StoryTracker` → `BoardStore` + `SyncOrchestrator`.
- **Phase 5** pending: delete `StoryTracker` trait + `FakeTracker` + `OctocrabStoryTracker` once every handler is on board.

Today the bridge still uses tracker for most handlers; substrate is live alongside and ready to absorb them.
