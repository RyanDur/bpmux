import { describe, expect, inject, it } from 'vitest'

import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 — POST /api/stories/:n/labels mutates raw_labels atomically', () => {
  it('add+remove pair retags the story so the new label survives + the old does not', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7501,
        title: 'story to relabel',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Cassiopeia.200']
      }
    ])

    const mutationRes = await fetch(`${apiBase}/api/stories/7501/labels`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${authToken}`
      },
      body: JSON.stringify({
        add: ['iteration-Cassiopeia.50'],
        remove: ['iteration-Cassiopeia.200']
      })
    })
    expect(mutationRes.status).toBe(200)

    const boardRes = await fetch(`${apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${authToken}` }
    })
    const boardBody = (await boardRes.json()) as {
      stories: { id: number; labels: string[] }[]
    }
    const story = boardBody.stories.find((s) => s.id === 7501)
    expect(story).toBeDefined()
    expect(story?.labels).toContain('iteration-Cassiopeia.50')
    expect(story?.labels).not.toContain('iteration-Cassiopeia.200')
  }, 20_000)
})
