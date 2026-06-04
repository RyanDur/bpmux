import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'

describe('#1339 step 2h-4 — keyboard-reorder iteration-itself persists via iteration-order label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  const orderFor = (labels: readonly string[] | undefined): number | null => {
    if (!labels) return null
    for (const l of labels) {
      const m = l.match(/^iteration-order\.(\d+)$/)
      if (m) return parseInt(m[1], 10)
    }
    return null
  }

  it('Ctrl+Shift+ArrowUp on Iteration Draco (high iteration-order) lowers its order below Iteration Tucana and removes the old label', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8401,
        title: 'Iteration Tucana',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.50000']
      },
      {
        number: 8402,
        title: 'Iteration Draco',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.60000']
      },
      {
        number: 8410,
        title: 'tucana-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Tucana.100']
      },
      {
        number: 8411,
        title: 'draco-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Draco.100']
      }
    ])

    rendered = renderApp()

    const dracoCard = await rendered.findByTestId('board-iteration-8402', undefined, {
      timeout: 15_000
    })
    const dracoRow = dracoCard.closest('.board-iteration-row') as HTMLElement
    const dracoGrip = within(dracoRow).getByTestId('iteration-grip') as HTMLButtonElement
    expect(dracoGrip).toBeTruthy()

    const user = userEvent.setup()
    await pressReorderShortcut(user, dracoGrip, 'ArrowUp')

    await waitFor(
      async () => {
        const boardRes = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const boardBody = (await boardRes.json()) as {
          stories: { number: number; labels: string[] }[]
        }
        const draco = boardBody.stories.find((s) => s.number === 8402)
        const tucana = boardBody.stories.find((s) => s.number === 8401)
        const dracoOrder = orderFor(draco?.labels)
        const tucanaOrder = orderFor(tucana?.labels)
        expect(dracoOrder).not.toBeNull()
        expect(tucanaOrder).not.toBeNull()
        expect(dracoOrder).toBeLessThan(tucanaOrder!)
        expect(draco?.labels).not.toContain('iteration-order.60000')
      },
      { timeout: 10_000, interval: 200 }
    )

    const announce = rendered.getByTestId('board-reorder-announce')
    expect(announce.textContent).toMatch(
      /^Moved Iteration Draco to position \d+ of \d+ among iterations$/
    )
  }, 30_000)
})
