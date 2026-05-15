import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1297 Slice A — iteration appears in icebox as a positional marker', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'iteration story renders in the icebox column with data-story-type="iteration"',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7001,
          title: 'Iteration Cassiopeia',
          story_type: 'iteration',
          initial_state: 'icebox'
        }
      ])

      rendered = renderApp()

      const iterationRow = await rendered.findByTestId('board-story-7001', undefined, {
        timeout: 15_000
      })
      expect(iterationRow.getAttribute('data-story-type')).toBe('iteration')
      expect(iterationRow.textContent).toContain('Iteration Cassiopeia')
    },
    25_000
  )
})
