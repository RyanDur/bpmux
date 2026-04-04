# Replaceable Bot Architecture

How should bpmux design its built-in bot so users can swap it out? Research into how established platforms solve "default but replaceable" components.

## 1. The Kubernetes Pattern

Kubernetes ships defaults for nearly every subsystem — scheduler, DNS, ingress, container runtime, networking — and each is replaceable through a well-defined interface. Three distinct replacement mechanisms exist, escalating in coupling:

**Webhook model (loosest coupling).** The core component makes HTTP callouts to an external service. The legacy scheduler extender used this: kube-scheduler calls a remote HTTP endpoint for filter/prioritize decisions. Low barrier to entry, but limited — only two extension points (filter and prioritize) were available through the webhook.

**Plugin framework (compiled in, but configurable).** The modern [scheduling framework](https://kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework/) defines 12+ extension points (preFilter, filter, score, reserve, permit, bind, etc.) as Go interfaces. Plugins are compiled into the scheduler binary but enabled/disabled via YAML configuration:

```yaml
profiles:
  - schedulerName: default-scheduler
    plugins:
      score:
        disabled:
          - name: PodTopologySpread
        enabled:
          - name: MyCustomPlugin
            weight: 2
```

The wildcard `- name: '*'` disables all default plugins at an extension point. Pods select their scheduler via `spec.schedulerName`.

**Binary plugin model (process boundary).** Kubelet discovers CSI storage plugins and CNI network plugins as external binaries on a well-known path. The [Container Runtime Interface](https://kubernetes.io/docs/concepts/containers/cri/) (CRI) uses gRPC over a Unix socket — kubelet acts as client, the runtime as server. Two protobuf services define the contract: `RuntimeService` (pod/container lifecycle) and `ImageService` (pull/inspect/remove images). The [cri-api](https://github.com/kubernetes/cri-api) repository is the canonical source.

**What makes the interfaces stable:**
- Protobuf/gRPC with versioned schemas (CRI)
- YAML configuration with named extension points (scheduler)
- A single selection mechanism per pod (`schedulerName`) or per cluster (CRI socket path)
- Clear capability boundaries — each extension point does one thing

**Relevance to bpmux:** The CRI model is closest to what bpmux needs. The bot is a separate process. The WebSocket protocol is the interface. The "socket path" equivalent is the WebSocket URL. The selection mechanism is configuration.

Sources:
- https://kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework/
- https://kubernetes.io/docs/reference/scheduling/config/
- https://kubernetes.io/docs/concepts/extend-kubernetes/
- https://kubernetes.io/docs/concepts/containers/cri/
- https://github.com/kubernetes/cri-api

## 2. VS Code's Extension Model

VS Code bundles built-in extensions for syntax highlighting, basic IntelliSense, Git, and more. Third-party extensions can override or augment these through several mechanisms:

**Contribution points.** Extensions declare capabilities statically in `package.json` — languages, themes, commands, views, debuggers. VS Code merges contributions with a defined priority: core first, then built-in extensions, then third-party extensions. Later contributions can shadow earlier ones.

**Language Server Protocol (LSP).** The [LSP](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) is VS Code's most important replacement interface. A language server declares capabilities during initialization (completions, hover, go-to-definition, etc.). When a third-party language server provides the same capability as a built-in, VS Code uses the third-party version. The capability negotiation happens at startup — server and client announce what they support.

**Activation events.** Extensions activate lazily based on triggers (file type opened, command invoked, view visible). Since VS Code 1.74, many activation events are implicit from contribution declarations.

**What makes it smooth vs painful:**
- Smooth: capability-based negotiation means replacements "just work" without explicit disable steps. You install a Rust extension and it provides completions; you do not need to disable a built-in Rust completer.
- Smooth: the extension marketplace handles discovery. Users search, install, done.
- Painful: when two extensions provide the same capability without clear priority, conflicts arise (multiple formatters, competing completion providers). VS Code mitigates this with settings like `editor.defaultFormatter`.
- Painful: no formal "I am replacing extension X" declaration. Replacement is implicit through capability overlap.

**Relevance to bpmux:** The "capability negotiation at connect time" pattern is interesting. A bot connecting via WebSocket could declare what it does (responds to messages, summarizes, etc.) and bpmux could route accordingly. But this is probably over-engineering for v1. The simpler lesson: make the default easy to install and the replacement require zero changes to bpmux itself.

Sources:
- https://code.visualstudio.com/api/language-extensions/language-server-extension-guide
- https://code.visualstudio.com/api/references/contribution-points
- https://code.visualstudio.com/api/references/activation-events

## 3. Docker's Runtime Model

Docker originally bundled its own container runtime. Kubernetes introduced the [Container Runtime Interface](https://kubernetes.io/blog/2016/12/container-runtime-interface-cri-in-kubernetes/) (CRI) to decouple from Docker specifically.

**How the interface is defined:**
- Two protobuf services: `RuntimeService` and `ImageService`
- Communication over gRPC on a Unix socket
- Kubelet is the client; the runtime is the server
- The socket path is the only configuration needed to swap runtimes

**Key RPCs in RuntimeService:** CreatePodSandbox, StopPodSandbox, RemovePodSandbox, CreateContainer, StartContainer, StopContainer, RemoveContainer, ContainerStats, GetContainerEvents.

**Key RPCs in ImageService:** PullImage, ImageStatus, RemoveImage, ListImages.

Docker did not implement CRI directly. Kubernetes used a shim (dockershim) as a bridge. When dockershim was removed in Kubernetes 1.24, users migrated to containerd or CRI-O — both native CRI implementations. The lesson: the interface must be clean enough that the default implementation does not get special treatment. Dockershim was technical debt from the default getting privileged access.

**Relevance to bpmux:** The WebSocket protocol is bpmux's CRI. The built-in bot must connect through exactly the same WebSocket interface that any third-party bot would use. No backdoor, no special internal channel. If `bpmux bot` uses any capability not available to external bots, that is a dockershim — it will become debt.

Sources:
- https://kubernetes.io/blog/2016/12/container-runtime-interface-cri-in-kubernetes/
- https://github.com/kubernetes/cri-api
- https://opencontainers.org/

## 4. The WebSocket Interface as the Bot API

bpmux already has a WebSocket protocol (`src/mcp/protocol.rs`). The current contract:

**Client -> Server:**
- `{"type": "identify", "name": "<name>"}` — must be first message, 5-second timeout
- `{"type": "send", "text": "<message>"}` — send a chat message
- `{"type": "ping"}` — keepalive

**Server -> Client:**
- `{"type": "welcome", "session_id": "<id>"}` — after successful identify
- `{"type": "message", "from": "<identity>", "text": "<text>", "ts": "<rfc3339>"}` — broadcast message
- `{"type": "error", "reason": "<reason>"}` — error notification
- `{"type": "pong"}` — ping response

**Identity assignment:** The server appends `:ws` to the name from identify, creating the identity string (e.g., `claude:ws`).

**What needs documenting for a replacement bot:**

1. **Connection lifecycle.** Connect to `ws://localhost:{port}/ws`. Send identify within 5 seconds. Receive welcome. Then bidirectional messaging. Connection drops are terminal (no resume protocol currently).

2. **Message format.** JSON, tagged with `"type"` field. All fields required. Max message size: 4096 bytes.

3. **Identity rules.** The name in identify becomes `{name}:ws` as the visible sender. Bot should choose a name that indicates what it is (e.g., `summarizer`, `reviewer`).

4. **Message flow.** After welcome, the bot receives all messages from all participants (including its own, echoed back). The bot sends messages by sending `{"type": "send", "text": "..."}`.

5. **No authentication currently.** Anyone who can reach the WebSocket endpoint can connect. This is fine for local-only; needs thought for networked scenarios.

6. **Rate limits.** None currently enforced. A misbehaving bot could flood the session. This should be documented as "undefined behavior" or have explicit limits.

7. **Reconnection.** Not supported. If the WebSocket drops, the bot must establish a new connection and re-identify. It will miss messages sent while disconnected. (Compare Discord, which has a resume protocol with sequence numbers.)

8. **Multiple connections.** The broadcast channel supports multiple WebSocket clients. A bot and a human's MCP tool can both be connected simultaneously.

## 5. Configuration for Swapping

How should users configure which bot to use?

**Precedent from Kubernetes, containerd, and others:** Configuration layers with clear precedence. CLI flags override config files override environment variables override defaults.

**Recommended approach for bpmux:**

The simplest UX is: the built-in bot is a separate process (`bpmux bot`), and replacing it means running a different process instead. No configuration needed in bpmux itself — the server does not care who connects via WebSocket.

But users need a way to say "start the default bot automatically" or "do not start a bot, I will run my own." This belongs in `~/.bpmux/config.toml`:

```toml
[bot]
# "builtin" | "none" | "command:<shell command>"
provider = "builtin"

# Only used when provider = "builtin"
# name the bot identifies as
name = "assistant"
```

**CLI override:**
```bash
bpmux create --name ryan --bot none          # no bot
bpmux create --name ryan --bot builtin       # default
bpmux create --name ryan --bot "command:my-bot --port 4117"  # custom
```

**Environment variable:**
```bash
BPMUX_BOT=none bpmux create --name ryan
```

**Precedence:** CLI flag > env var > config.toml > default ("builtin").

**Why not more complex?** Kubernetes needs YAML because it manages clusters. bpmux manages a single session on a single machine. A TOML field and a CLI flag are sufficient. The bot connects via WebSocket — bpmux does not need to know anything about the bot's implementation, only whether to start the default one.

## 6. The Bot as a Separate Process

If the built-in bot is `bpmux bot`, it is already architecturally separated:

- It connects via the same WebSocket interface as any external bot
- It runs as a child process (or user-started process)
- Replacing it means: do not run `bpmux bot`, run your replacement instead

**Is that simple enough?**

Yes, with one addition: `bpmux create` should optionally auto-start the bot as a child process and auto-stop it when the session ends. This is convenience, not coupling. The bot process does not get special access — it connects over WebSocket like anything else.

**Does bpmux need a bot management layer?**

No. A management layer (health checks, restart policies, resource limits) adds complexity without proportional value for a local development tool. If the bot crashes, the user sees it stop responding and restarts it. This is the same experience as any other local dev tool crashing.

The temptation to build a supervisor comes from server-side thinking. bpmux is a desktop tool. The user is the supervisor.

If the `command:` provider is used, bpmux could do basic lifecycle management: start on session create, stop on session end, restart on crash with backoff. But this is a v2 concern. For v1, start/stop is sufficient.

## 7. Multi-Bot Support

The WebSocket broadcast channel already supports multiple clients. Running the default bot AND a custom bot simultaneously is technically possible today.

**When this is useful:**
- Default bot handles general chat assistance; specialized bot handles code review
- One bot summarizes; another bot translates
- Gradual migration: run old and new bot side-by-side during transition

**Coordination problems:**
- **Echo loops.** Bot A responds to a message. Bot B sees that response and responds to it. Bot A sees that and responds again. This is the most likely failure mode. Mitigation: bots should filter messages by sender identity and not respond to other bots (or to themselves).
- **Duplicate responses.** Both bots respond to the same human message with similar content. Annoying but not dangerous. Mitigation: bots should have distinct roles/triggers.
- **Ordering.** No guaranteed ordering of bot responses. If both bots respond to the same prompt, the order depends on processing time. This is fine for a chat — humans deal with unordered concurrent messages naturally.
- **Resource contention.** Multiple bots calling the same LLM API multiply costs. Not a bpmux problem, but worth documenting.

**The protocol already handles this.** Each WebSocket client gets its own identity (`bot-a:ws`, `bot-b:ws`). Messages include the sender identity, so bots can filter. No additional coordination mechanism is needed at the protocol level.

**Recommendation:** Support multi-bot from day one (it is already supported by the protocol). Document the echo-loop risk. Do not build orchestration — let users coordinate their bots themselves.

## 8. Documentation as Interface

How Discord and Slack document their bot APIs, and what bpmux should learn from them.

### Discord's Gateway Documentation

Discord's [gateway documentation](https://docs.discord.com/developers/events/gateway) is comprehensive and follows a clear structure:

1. **Connection lifecycle** as a numbered sequence with a flowchart
2. **Payload format** — every message has `op` (opcode), `d` (data), `s` (sequence), `t` (event type)
3. **Opcodes** — enumerated table with send/receive direction
4. **Events** — full reference of every event type with payload schemas
5. **Intents** — bitwise flags controlling which events you receive
6. **Rate limits** — explicit: 120 events per connection per 60 seconds, 4096 bytes max
7. **Encoding and compression** — JSON or ETF, optional zlib/zstd
8. **Sharding** — formula for distributing across connections
9. **Error codes** — close codes with descriptions and whether to reconnect

Discord's docs are thorough but assume you will use a community library rather than implement the protocol directly.

### Slack's Socket Mode Documentation

Slack's [Socket Mode documentation](https://docs.slack.dev/apis/events-api/using-socket-mode/) is more opinionated:

1. **Setup steps** — enable Socket Mode, generate app-level token, subscribe to events
2. **Connection** — call `apps.connections.open` with token, get WebSocket URL, connect
3. **Payload format** — same as Events API but with `envelope_id` for acknowledgment
4. **Acknowledgment** — you must acknowledge each event by its envelope ID
5. **Limits** — max 10 concurrent WebSocket connections
6. **SDK-first approach** — docs push the Bolt framework heavily, raw protocol docs are secondary

Slack's approach: make the SDK the primary interface, document the raw protocol as a reference.

### Minimum Viable Bot Developer Documentation for bpmux

Based on these examples, bpmux needs a single document covering:

**1. Quick start (30 seconds to first bot)**
```
1. Connect: ws://localhost:4117/ws
2. Send: {"type": "identify", "name": "my-bot"}
3. Receive: {"type": "welcome", "session_id": "..."}
4. Receive messages: {"type": "message", "from": "...", "text": "...", "ts": "..."}
5. Send messages: {"type": "send", "text": "Hello from my bot"}
```

**2. Protocol reference**
- Client messages: identify, send, ping (with JSON schemas)
- Server messages: welcome, message, error, pong (with JSON schemas)
- Connection rules: identify within 5 seconds, max 4096 bytes, no resume

**3. Identity**
- Name becomes `{name}:ws`
- Choose descriptive names: `summarizer`, `reviewer`, `translator`
- Multiple bots can connect simultaneously with different names

**4. Behavioral guidelines**
- Filter messages by `from` to avoid echo loops
- Do not respond to your own messages
- Do not respond to other bots unless that is your explicit purpose
- Handle disconnections by reconnecting and re-identifying

**5. Example bot** — a minimal working bot in Python or Node.js, under 50 lines, that echoes messages. This is more valuable than pages of specification.

**What bpmux does NOT need (yet):**
- Intents/event filtering (all clients get all messages)
- Authentication (local-only for now)
- Sharding (single session, single server)
- Rate limit tiers (one limit for all clients)
- Versioned API (one version, breaking changes are fine at this stage)

## Synthesis: Recommendations for bpmux

**The WebSocket protocol IS the bot API.** No separate bot API needed. Document the existing protocol, and you have documented the bot interface.

**The built-in bot must eat its own dog food.** `bpmux bot` must connect via the same WebSocket interface as any third-party bot. No internal shortcuts. This is the lesson from Docker's dockershim: special treatment of the default creates debt.

**Configuration is minimal.** A `[bot]` section in config.toml with `provider = "builtin" | "none"` and a `--bot` CLI flag. That is the entire UX for swapping.

**The bot is a process, not a plugin.** Process boundary gives clean separation. No shared memory, no compiled-in interface, no version coupling. The bot can be written in any language that speaks WebSocket + JSON.

**Multi-bot works for free.** The broadcast channel already supports it. Document the echo-loop risk and move on.

**Documentation is the real interface.** A clear quick-start guide with a working example bot is more important than a formal specification. Discord and Slack both prove that developers reach for the quick start, not the spec. Write the quick start first.

**Do not build what you do not need.** No bot supervisor, no capability negotiation, no plugin registry, no marketplace. A WebSocket URL, a JSON protocol, and a config flag. That is the interface.

Sources:
- https://kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework/
- https://kubernetes.io/docs/reference/scheduling/config/
- https://kubernetes.io/docs/concepts/extend-kubernetes/
- https://kubernetes.io/docs/concepts/containers/cri/
- https://github.com/kubernetes/cri-api
- https://kubernetes.io/blog/2016/12/container-runtime-interface-cri-in-kubernetes/
- https://opencontainers.org/
- https://code.visualstudio.com/api/language-extensions/language-server-extension-guide
- https://code.visualstudio.com/api/references/contribution-points
- https://code.visualstudio.com/api/references/activation-events
- https://docs.discord.com/developers/events/gateway
- https://docs.slack.dev/apis/events-api/using-socket-mode/
- https://docs.slack.dev/apis/events-api/comparing-http-socket-mode/
