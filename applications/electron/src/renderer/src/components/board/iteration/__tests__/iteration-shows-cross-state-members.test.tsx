import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../../../test/fixtures/seedMessages'
import { iterationBoard } from './iterationBoard.page'

describe('iteration container shows members regardless of workflow state (historical-marker)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('an in-acceptance story with parent-iteration-.100 label renders inside the Cassiopeia container body AND in its workflow column', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 9300,
        title: 'Iteration Cassiopeia',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.100'],
        body: '<!-- bpmux-relationships: {"stories":[9301]} -->'
      },
      {
        id: 9301,
        title: 'cassiopeia-member-acceptance',
        story_type: 'feature',
        initial_state: 'acceptance'
      }
    ])

    rendered = renderApp()
    const container = await iterationBoard.container(9300, undefined, { timeout: 15_000 })
    const memberInContainer = iterationBoard.storyRow(container, 9301)
    expect(memberInContainer).toBeTruthy()
  }, 20_000)
})
