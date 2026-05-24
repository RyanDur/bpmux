import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../test/fixtures/pressReorderShortcut'

describe('#1339 step 2h-2 — keyboard-reorder announces container-context for screen readers', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('Ctrl+Shift+ArrowUp on a Phoenix member updates the live-region with title, new position, total, and container name', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7901,
        title: 'Iteration Phoenix',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7911,
        title: 'phoenix-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7901.100']
      },
      {
        number: 7912,
        title: 'phoenix-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7901.200']
      },
      {
        number: 7913,
        title: 'phoenix-charlie',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7901.300']
      }
    ])

    rendered = renderApp()

    const container = await rendered.findByTestId('board-iteration-7901', undefined, {
      timeout: 15_000
    })
    const charlieRow = within(container).getByTestId('board-story-7913') as HTMLLIElement
    const charlieButton = within(charlieRow).getByTestId(
      'board-iteration-member-button'
    ) as HTMLButtonElement

    const user = userEvent.setup()
    await pressReorderShortcut(user, charlieButton, 'ArrowUp')

    const announce = rendered.getByTestId('board-reorder-announce')
    await waitFor(
      () => {
        expect(announce.textContent).toBe(
          'Moved phoenix-charlie to position 2 of 3 in Iteration Phoenix'
        )
      },
      { timeout: 5_000, interval: 100 }
    )
  }, 25_000)
})
