import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'
import { keyboardReorder } from '../board/keyboardReorder.page'

describe('#1339 step 2h-4 — keyboard-reorder iteration-itself persists via iteration-order label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

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

    const dracoGrip = await keyboardReorder.iterationGrip(8402, { timeout: 15_000 })
    expect(dracoGrip).toBeTruthy()

    const user = userEvent.setup()
    await pressReorderShortcut(user, dracoGrip, 'ArrowUp')

    await waitFor(
      async () => {
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { number: number; iteration_order: number | null }[]
        }
        const orderOf = (n: number): number | null =>
          iterBody.iterations.find((s) => s.number === n)?.iteration_order ?? null
        const dracoOrder = orderOf(8402)
        const tucanaOrder = orderOf(8401)
        expect(dracoOrder).not.toBeNull()
        expect(tucanaOrder).not.toBeNull()
        expect(dracoOrder).toBeLessThan(tucanaOrder!)
        expect(dracoOrder).not.toBe(60000)
      },
      { timeout: 10_000, interval: 200 }
    )

    const announce = keyboardReorder.announce()
    expect(announce.textContent).toMatch(
      /^Moved Iteration Draco to position \d+ of \d+ among iterations$/
    )
  }, 30_000)
})
