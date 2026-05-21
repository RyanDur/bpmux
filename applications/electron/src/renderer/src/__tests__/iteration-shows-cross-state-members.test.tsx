import { afterEach, describe, expect, inject, it } from 'vitest'
import { within } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1297 — iteration container shows members regardless of workflow state (historical-marker)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('a delivered story with iteration-Cassiopeia.100 label renders inside the Cassiopeia container body AND in its workflow column', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 9300,
        title: 'Iteration Cassiopeia',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.100']
      },
      {
        number: 9301,
        title: 'cassiopeia-member-delivered',
        story_type: 'feature',
        initial_state: 'delivered',
        labels: ['iteration-Cassiopeia.100']
      }
    ])

    rendered = renderApp()
    const container = await rendered.findByTestId('board-iteration-9300', undefined, {
      timeout: 15_000
    })
    const memberInContainer = within(container).getByTestId('board-story-9301')
    expect(memberInContainer).toBeTruthy()
  }, 20_000)
})
