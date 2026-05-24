import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'

describe('#1339 step 2h-3 — keyboard-reorder edge-cases announce intent to SR users', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('Ctrl+Shift+ArrowUp on first-position Andromeda member announces "already at position 1" and does not mutate labels', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8201,
        title: 'Iteration Andromeda',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 8211,
        title: 'andromeda-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#8201.100']
      },
      {
        number: 8212,
        title: 'andromeda-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#8201.200']
      }
    ])

    rendered = renderApp()

    const container = await rendered.findByTestId('board-iteration-8201', undefined, {
      timeout: 15_000
    })
    const alphaRow = within(container).getByTestId('board-story-8211') as HTMLLIElement
    const alphaButton = within(alphaRow).getByTestId(
      'board-iteration-member-button'
    ) as HTMLButtonElement

    const user = userEvent.setup()
    await pressReorderShortcut(user, alphaButton, 'ArrowUp')

    const announce = rendered.getByTestId('board-reorder-announce')
    await waitFor(
      () => {
        expect(announce.textContent).toBe(
          'andromeda-alpha is already at position 1 in Iteration Andromeda'
        )
      },
      { timeout: 5_000, interval: 100 }
    )

    const boardRes = await fetch(`${apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${authToken}` }
    })
    const boardBody = (await boardRes.json()) as {
      stories: {
        number: number
        iteration_membership: { iteration_number: number; position: number } | null
      }[]
    }
    const alpha = boardBody.stories.find((s) => s.number === 8211)
    expect(alpha?.iteration_membership).toEqual({ iteration_number: 8201, position: 100 })
  }, 25_000)

  it('Ctrl+Shift+ArrowDown on last-position Perseus member announces "already at last position" and does not mutate labels', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8301,
        title: 'Iteration Perseus',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 8311,
        title: 'perseus-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#8301.100']
      },
      {
        number: 8312,
        title: 'perseus-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#8301.200']
      }
    ])

    rendered = renderApp()

    const container = await rendered.findByTestId('board-iteration-8301', undefined, {
      timeout: 15_000
    })
    const bravoRow = within(container).getByTestId('board-story-8312') as HTMLLIElement
    const bravoButton = within(bravoRow).getByTestId(
      'board-iteration-member-button'
    ) as HTMLButtonElement

    const user = userEvent.setup()
    await pressReorderShortcut(user, bravoButton, 'ArrowDown')

    const announce = rendered.getByTestId('board-reorder-announce')
    await waitFor(
      () => {
        expect(announce.textContent).toBe(
          'perseus-bravo is already at last position in Iteration Perseus'
        )
      },
      { timeout: 5_000, interval: 100 }
    )

    const boardRes = await fetch(`${apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${authToken}` }
    })
    const boardBody = (await boardRes.json()) as {
      stories: {
        number: number
        iteration_membership: { iteration_number: number; position: number } | null
      }[]
    }
    const bravo = boardBody.stories.find((s) => s.number === 8312)
    expect(bravo?.iteration_membership).toEqual({ iteration_number: 8301, position: 200 })
  }, 25_000)
})
