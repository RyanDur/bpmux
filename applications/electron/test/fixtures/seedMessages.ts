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

export async function resetMessages(apiBase: string, authToken: string): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/reset_messages`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${authToken}` }
  })
  if (!res.ok) {
    throw new Error(`reset_messages POST failed: ${res.status} ${await res.text()}`)
  }
}

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

export interface SeedStory {
  number: number
  title: string
  body?: string
  initial_state?: 'icebox' | 'ready' | 'in-progress' | 'desk-check' | 'delivered' | 'accepted'
  story_type?: 'feature' | 'bug' | 'chore' | 'epic' | 'iteration'
  iteration_id?: number
}

export async function seedStories(
  apiBase: string,
  authToken: string,
  stories: SeedStory[]
): Promise<void> {
  const res = await fetch(`${apiBase}/api/test/seed_stories`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${authToken}`
    },
    body: JSON.stringify({ stories })
  })
  if (!res.ok) {
    throw new Error(`seed_stories POST failed: ${res.status} ${await res.text()}`)
  }
}
