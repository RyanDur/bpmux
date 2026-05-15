import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 Slice B — iteration marker drag-reshape contracts', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'open iteration marker exposes draggable=true so positional reshape is gesture-enabled',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        { number: 8101, title: 'Story above marker', initial_state: 'icebox' },
        {
          number: 8102,
          title: 'Iteration Cygnus',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        { number: 8103, title: 'Story below marker', initial_state: 'icebox' }
      ])

      rendered = renderApp()

      const marker = await rendered.findByTestId('board-story-8102', undefined, { timeout: 15_000 })
      expect(marker.getAttribute('data-story-type')).toBe('iteration')
      expect(marker.getAttribute('draggable')).toBe('true')
    },
    25_000
  )
})
