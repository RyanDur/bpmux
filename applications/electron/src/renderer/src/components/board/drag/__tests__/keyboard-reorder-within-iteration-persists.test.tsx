import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../../../../test/fixtures/pressReorderShortcut'
import { keyboardReorder } from './keyboardReorder.page'

describe('keyboard-reorder within iteration container persists via label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('Ctrl+Shift+ArrowDown on Lacerta member at position 1 moves it one slot down (new persisted order [bravo, alpha, charlie])', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7801,
        title: 'Iteration Lacerta',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7811,7812,7813]} -->'
      },
      {
        id: 7811,
        title: 'lacerta-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7812,
        title: 'lacerta-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7813,
        title: 'lacerta-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await keyboardReorder.iterationCard(7801, { timeout: 15_000 })
    const alphaButton = keyboardReorder.memberButton(container, 7811)

    const user = userEvent.setup()
    await pressReorderShortcut(user, alphaButton, 'ArrowDown')

    await waitFor(
      async () => {
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { id: number; stories: number[] }[]
        }
        const lacerta = iterBody.iterations.find((i) => i.id === 7801)
        expect(lacerta?.stories).toEqual([7812, 7811, 7813])
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)

  it('Ctrl+Shift+ArrowUp on Centaurus member at last position moves it one slot up (new persisted order [alpha, charlie, bravo])', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7701,
        title: 'Iteration Centaurus',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7711,7712,7713]} -->'
      },
      {
        id: 7711,
        title: 'centaurus-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7712,
        title: 'centaurus-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7713,
        title: 'centaurus-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await keyboardReorder.iterationCard(7701, { timeout: 15_000 })
    expect(keyboardReorder.memberOrder(container)).toEqual([7711, 7712, 7713])

    const charlieButton = keyboardReorder.memberButton(container, 7713)
    expect(charlieButton).toBeTruthy()

    const user = userEvent.setup()
    await pressReorderShortcut(user, charlieButton, 'ArrowUp')

    await waitFor(
      async () => {
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { id: number; stories: number[] }[]
        }
        const centaurus = iterBody.iterations.find((i) => i.id === 7701)
        expect(centaurus?.stories).toEqual([7711, 7713, 7712])
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
