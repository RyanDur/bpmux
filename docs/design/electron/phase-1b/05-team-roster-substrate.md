# phase-1b team-roster substrate (#402)

Ryan 2026-04-23 12:08 UTC with screenshot: *"keep losing the groups
and directs."* Sidebar shows only `designer` under directs; dev + pm
absent despite being connected + active today.

Root cause: `deriveTeamRoles(messages, viewer)` (#393 AC1) derives
membership from OBSERVED messages. When the hydration window doesn't
include recent messages from a role, that role drops out of the
sidebar. Fundamentally team-shape ≠ who-has-spoken-recently.

## 3-field spec header

**user-job:** When Ryan opens Electron (or has it open), he needs the
sidebar to accurately show every connected agent as a direct-DM
candidate + include every combinable subset as a group, because the
sidebar is his primary affordance for scoping conversations — if a
teammate isn't visible there, Ryan literally cannot DM them via click.
Membership must be stable across hydration windows + filter states.

**observation-falsifier:** I would reject if:
- a bridge-connected agent (dev / pm / designer / ops / etc.) is absent
  from the sidebar directs section for ANY reason other than being
  the viewer or a system sender
- a combinatorial group involving a connected agent is absent from
  the groups section
- the sidebar's visible directs/groups set FLUCTUATES as
  messages-hydration-window-changes, filter-state-changes, or
  scroll-position-changes
- membership derives from message presence instead of connection
  presence

**team-shape-independence:** source is bridge-connection-roster (who's
joined the session), not hardcoded role names. A fresh role joining
via MCP appears in the sidebar on its next connection heartbeat
without code change.

## architectural shift

**Before (#393 AC1 / #386 / #399 — current):**

```
message stream (chat_only=200 + activity=50) →
  deriveTeamRoles(messages, viewer) →
  deriveDirectRoles / deriveCombinableRoles →
  sidebar entries
```

Problem: membership follows observed-history. Narrow window = missing
roles. "Keep losing the groups and directs" is exactly this class.

**After (#402 / #1246):**

```
bridge-connection-roster (backend tracks joined agents) →
  useTeamRoster hook (refetches on WS-reconnect) →
  deriveDirectRolesFromRoster / deriveCombinableRolesFromRoster
  (operate on roster only; no message-history fallback) →
  sidebar entries

observed messages →
  usePresence (still operates on recent messages; drives busy/idle
  dot COLOR, not membership)
```

Two axes:
- **membership (roster-authority):** who's on the team
- **activity (message-recency):** who's actively talking/working

Only the second depends on observed messages. Membership is
connection-level.

## required backend substrate

New or existing backend surface to expose the roster. Two options:

### (a) REST endpoint: `GET /api/roster`

Returns currently-connected agent identities:

```json
{
  "roster": [
    { "identity": "ryan:electron", "role": "ryan", "connected_since": "..." },
    { "identity": "dev:claude-code", "role": "dev", "connected_since": "..." },
    { "identity": "pm:claude-code", "role": "pm", "connected_since": "..." },
    { "identity": "designer:claude-code", "role": "designer", "connected_since": "..." }
  ],
  "fetched_at": "..."
}
```

Frontend polls at ~10s cadence OR consumes roster updates via WS
broadcast (preferred — push-based like `BOARD_INVALIDATE`).

### (b) WS broadcast marker: `ROSTER_UPDATE_MARKER`

When an agent joins / disconnects, bridge broadcasts a
roster-update-marker. Frontend refetches roster via (a) endpoint on
receipt. Push-based membership updates, same pattern as
`CI_STATE_UPDATE_MARKER` / `BOARD_INVALIDATE_MARKER`.

### dev-lane choice

Option (b) with (a) as the authoritative fetch target. Consistent with
existing push-subscription substrate. Initial connection does one
roster fetch at hydration; subsequent updates via marker +
invalidation.

## required frontend substrate

### `useTeamRoster` hook

```ts
interface RosterEntry {
  identity: string    // e.g., 'dev:claude-code'
  role: string        // short-name e.g., 'dev'
  connectedSince: string
  isSystem: boolean   // bpmux is isSystem=true; agents false
}

export function useTeamRoster(apiBase: string, authToken: string, messages: ChatMessage[]): RosterEntry[]
```

Lifecycle:
- Mount: fetch `/api/roster` once
- Subscribe to `messages` stream for `ROSTER_UPDATE_MARKER` → refetch
- Return current roster

### `deriveRoles.ts` signature changes

```ts
// Before:
deriveTeamRoles(messages, viewer) → string[]
deriveDirectRoles(messages, viewer) → string[]
deriveCombinableRoles(messages, viewer) → string[]

// After (#1246):
deriveTeamRolesFromRoster(roster) → string[]
deriveDirectRolesFromRoster(roster, viewer) → string[]
deriveCombinableRolesFromRoster(roster, viewer) → string[]
```

Inputs shift from `messages` to `roster`. Filter logic stays the same
(system-sender detection via `entry.isSystem`, viewer-exclusion via
`viewer`). No literal name strings — team-shape-independence preserved
at the helper layer.

### Sidebar.tsx wires through

```ts
const roster = useTeamRoster({ apiBase, authToken, mirrorRoster, wsStatus })
const directRoles = useMemo(
  () => deriveDirectRolesFromRoster(roster, RENDERER_IDENTITY),
  [roster]
)
const combinableRoles = useMemo(
  () => deriveCombinableRolesFromRoster(roster, RENDERER_IDENTITY),
  [roster]
)
// usePresence stays the same — consumes messages for activity signal
const presence = usePresence(messages, directRoles)
```

## migration path

1. Backend: ship `/api/roster` endpoint + `ROSTER_UPDATE_MARKER`
   broadcast on connect/disconnect events
2. Frontend: ship `useTeamRoster` hook
3. `deriveRoles.ts`: add new signatures (don't break old ones yet)
4. Sidebar.tsx: swap to roster-based calls
5. MessageList.tsx (avatar-hover + palette lookups): swap to roster
6. Retire observed-message-based derivation where unused; keep where
   it serves activity/presence

Can ship as one story (full-vertical) OR 2-story split (backend
+ frontend). Pm's scope call.

## acceptance criteria

1. With 4 agents connected (ryan + dev + pm + designer), sidebar
   shows dev + pm + designer in directs regardless of which agent has
   most-recently spoken.
2. Sidebar membership unchanged when Ryan scrolls through history
   (hydration window fluctuations).
3. Sidebar membership unchanged when filter state toggles.
4. A 5th agent (ops) joining via MCP appears in directs within
   connection-heartbeat latency (~seconds, not waiting to speak).
5. An agent disconnecting disappears from directs within heartbeat
   latency.
6. bpmux (system) never appears in directs or groups.
7. Ryan (viewer) never appears as a direct-DM target.
8. Combinatorial groups enumerate subsets of 2+ connected agents
   (excluding owner + system).
9. Presence dot color (busy/active/idle) still derives from recent
   observed messages — decoupled from membership.
10. Playwright test: fixture with roster endpoint mocked (dev + pm +
    designer); verify all 3 render in directs + 4 groups enumerate;
    then remove pm from roster fixture + refire — sidebar updates.

## out of scope

- Persistent roster across session-restart (current scope: in-session
  roster only; bridge knows who's currently connected)
- Privacy controls on roster (who-can-see-whom) — future if needed
- Multi-device awareness (same role with multiple connections) —
  roster treats role as the unit, not individual device-connections
- Full retirement of observed-message-derived `deriveTeamRoles` —
  stays as activity-inference substrate; #402 adds the roster-authority
  layer on top

## desk-check walk script

1. 4 agents connected (ryan + dev + pm + designer). Open Electron,
   verify sidebar shows dev + pm + designer in directs + 4
   combinatorial groups.
2. Quietly, dev + pm stop sending messages for ~5 minutes. Verify
   sidebar STILL shows dev + pm + designer unchanged.
3. Ryan scrolls through chat history. Verify sidebar unchanged.
4. Ryan filters chat to system-only. Verify sidebar unchanged.
5. Spawn ops agent via MCP. Verify ops appears in directs within
   seconds.
6. Disconnect pm (kill MCP session). Verify pm disappears from
   directs within heartbeat latency.
7. bpmux NEVER appears in directs or groups during any of above.

Walk pass = observation-falsifier doesn't fire at any step.
