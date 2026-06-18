import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'
import { keyboardReorder } from '../board/keyboardReorder.page'

describe('#1339 step 2h-1 — keyboard-reorder within iteration container persists via label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('Ctrl+Shift+ArrowDown on Lacerta member at position 100 mutates its label to position 250 (between existing 200 and 300)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7801,
        title: 'Iteration Lacerta',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7811,
        title: 'lacerta-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7801.100']
      },
      {
        number: 7812,
        title: 'lacerta-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7801.200']
      },
      {
        number: 7813,
        title: 'lacerta-charlie',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7801.300']
      }
    ])

    rendered = renderApp()

    const container = await keyboardReorder.iterationCard(7801, { timeout: 15_000 })
    const alphaButton = keyboardReorder.memberButton(container, 7811)

    const user = userEvent.setup()
    await pressReorderShortcut(user, alphaButton, 'ArrowDown')

    await waitFor(
      async () => {
        const boardRes = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const boardBody = (await boardRes.json()) as {
          stories: {
            number: number
            iteration_membership: { iteration_number: number; position: number } | null
          }[]
        }
        const alpha = boardBody.stories.find((s) => s.number === 7811)
        expect(alpha?.iteration_membership).toEqual({ iteration_number: 7801, position: 250 })
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)

  it('Ctrl+Shift+ArrowUp on Centaurus member at position 300 mutates its label to position 150 (between existing 100 and 200)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7701,
        title: 'Iteration Centaurus',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7711,
        title: 'centaurus-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7701.100']
      },
      {
        number: 7712,
        title: 'centaurus-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7701.200']
      },
      {
        number: 7713,
        title: 'centaurus-charlie',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7701.300']
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
        const boardRes = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const boardBody = (await boardRes.json()) as {
          stories: {
            number: number
            iteration_membership: { iteration_number: number; position: number } | null
          }[]
        }
        const charlie = boardBody.stories.find((s) => s.number === 7713)
        expect(charlie?.iteration_membership).toEqual({ iteration_number: 7701, position: 150 })
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
