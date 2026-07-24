import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../test/fixtures/seedMessages'
import { iterationBoard } from './iterationBoard.page'

describe('closed iteration renders "(closed)" badge as JSX sibling (no strikethrough inheritance)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('closed iteration row renders a .board-iteration-closed-badge with text "(closed)" + data-closed="true" on the card', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 9201,
        title: 'Iteration RetiredOne',
        story_type: 'iteration',
        initial_state: 'icebox',
        is_closed: true
      }
    ])

    rendered = renderApp()
    const row = await iterationBoard.container(9201, undefined, { timeout: 15_000 })
    expect(row.getAttribute('data-closed')).toBe('true')
    const badge = iterationBoard.closedBadge(row)
    expect(badge).toBeTruthy()
    expect(badge?.textContent).toBe('(closed)')
  }, 20_000)
})
