// #1306 Epic #1247 slice 4.1: shared seed-messages helper.
//
// Extracted from #1304's archetype test-file so the human-message-*
// sub-batch (and subsequent batch-4.1 families) share one substrate
// instead of N copies. Calls /api/test/seed_messages (BPMUX_TEST_SEED_API
// gated; mounted in globalSetup.ts via the same env-var). Per Playwright
// launch.ts:242 the route accepts `{ messages: Message[] }`; this
// helper preserves the same wire format so the migration recipe stays
// mechanical (Playwright `seedMessages: [...]` opt → this helper call).

export interface SeedMessage {
  from: string
  text: string
  ts: string
  message_type: string
}

export async function seedMessages(
  apiBase: string,
  authToken: string,
  messages: SeedMessage[]
): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/seed_messages`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${authToken}`
    },
    body: JSON.stringify({ messages })
  })
  if (!res.ok) {
    throw new Error(`seed_messages POST failed: ${res.status} ${await res.text()}`)
  }
}

// #1306 slice 4.1: wipe message-store between tests so seeded state
// doesn't leak across cases sharing the per-run backend (vitest
// globalSetup). Calls /api/test/reset_messages (BPMUX_TEST_SEED_API
// gated; mounted in server.rs alongside the seed routes).
export async function resetMessages(apiBase: string, authToken: string): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/reset_messages`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${authToken}` }
  })
  if (!res.ok) {
    throw new Error(`reset_messages POST failed: ${res.status} ${await res.text()}`)
  }
}

// #1335 slice 4.1.B: seed roster identities (sister to roster_seed.json
// filesystem-write in launch.ts). Each entry is a full identity string
// (e.g., "dev:claude-code"); the renderer's useTeamRoster fetches
// /api/roster on mount + receives mirror updates and surfaces these as
// sidebar-direct-<role> buttons.
export async function seedRoster(
  apiBase: string,
  authToken: string,
  identities: string[]
): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/seed_roster`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${authToken}`
    },
    body: JSON.stringify({ identities })
  })
  if (!res.ok) {
    throw new Error(`seed_roster POST failed: ${res.status} ${await res.text()}`)
  }
}
