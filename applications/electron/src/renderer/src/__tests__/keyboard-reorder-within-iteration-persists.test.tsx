import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'

describe('#1339 step 2h-1 — keyboard-reorder within iteration container persists via label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'Ctrl+Shift+ArrowDown on Lacerta member at position 100 mutates its label to position 250 (between existing 200 and 300)',
    async () => {
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
          labels: ['iteration-Lacerta.100']
        },
        {
          number: 7812,
          title: 'lacerta-bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Lacerta.200']
        },
        {
          number: 7813,
          title: 'lacerta-charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Lacerta.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7801', undefined, {
        timeout: 15_000
      })
      const alphaRow = within(container).getByTestId('board-story-7811') as HTMLLIElement
      const alphaButton = within(alphaRow).getByTestId(
        'board-iteration-member-button'
      ) as HTMLButtonElement

      const user = userEvent.setup()
      await pressReorderShortcut(user, alphaButton, 'ArrowDown')

      await waitFor(
        async () => {
          const boardRes = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${authToken}` }
          })
          const boardBody = (await boardRes.json()) as {
            stories: { number: number; labels: string[] }[]
          }
          const alpha = boardBody.stories.find((s) => s.number === 7811)
          expect(alpha?.labels).toContain('iteration-Lacerta.250')
          expect(alpha?.labels).not.toContain('iteration-Lacerta.100')
        },
        { timeout: 10_000, interval: 200 }
      )
    },
    30_000
  )

  it(
    'Ctrl+Shift+ArrowUp on Centaurus member at position 300 mutates its label to position 150 (between existing 100 and 200)',
    async () => {
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
          labels: ['iteration-Centaurus.100']
        },
        {
          number: 7712,
          title: 'centaurus-bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Centaurus.200']
        },
        {
          number: 7713,
          title: 'centaurus-charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Centaurus.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7701', undefined, {
        timeout: 15_000
      })
      const orderBefore = within(container)
        .getAllByTestId(/^board-story-\d+$/)
        .map((el) => Number(el.getAttribute('data-iteration-member')))
      expect(orderBefore).toEqual([7711, 7712, 7713])

      const charlieRow = within(container).getByTestId('board-story-7713') as HTMLLIElement
      expect(charlieRow).toBeTruthy()
      const charlieButton = within(charlieRow).getByTestId(
        'board-iteration-member-button'
      ) as HTMLButtonElement
      expect(charlieButton).toBeTruthy()

      const user = userEvent.setup()
      await pressReorderShortcut(user, charlieButton, 'ArrowUp')

      await waitFor(
        async () => {
          const boardRes = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${authToken}` }
          })
          const boardBody = (await boardRes.json()) as {
            stories: { number: number; labels: string[] }[]
          }
          const charlie = boardBody.stories.find((s) => s.number === 7713)
          expect(charlie?.labels).toContain('iteration-Centaurus.150')
          expect(charlie?.labels).not.toContain('iteration-Centaurus.300')
        },
        { timeout: 10_000, interval: 200 }
      )
    },
    30_000
  )
})
