import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard } from './iterationBoard.page'

describe('iteration members render in the parent iteration stories order', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('member stories appear in the iteration body-JSON stories order inside the container body', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7401,
        title: 'Iteration Aquila',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7412,7411,7413]} -->'
      },
      {
        id: 7411,
        title: 'story alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7412,
        title: 'story bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7413,
        title: 'story charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(7401, undefined, { timeout: 15_000 })
    expect(iterationBoard.memberOrder(container)).toEqual([7412, 7411, 7413])
  }, 25_000)
})
