import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../../../test/fixtures/seedMessages'
import { pressReorderShortcut } from '../../../../../../../test/fixtures/pressReorderShortcut'
import { keyboardReorder } from './keyboardReorder.page'

describe('keyboard-reorder announces container-context for screen readers', () => {
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
        id: 7901,
        title: 'Iteration Phoenix',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7911,7912,7913]} -->'
      },
      {
        id: 7911,
        title: 'phoenix-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7912,
        title: 'phoenix-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7913,
        title: 'phoenix-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await keyboardReorder.iterationCard(7901, { timeout: 15_000 })
    const charlieButton = keyboardReorder.memberButton(container, 7913)

    const user = userEvent.setup()
    await pressReorderShortcut(user, charlieButton, 'ArrowUp')

    const announce = keyboardReorder.announce()
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
