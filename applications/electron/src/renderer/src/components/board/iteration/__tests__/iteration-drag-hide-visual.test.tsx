import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('dragged member carries data-dragging during drag gesture', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('onDragStart sets data-dragging="true" on the source row; onDragEnd clears it', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7801,
        title: 'Iteration Lyra',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7811]} -->'
      },
      {
        id: 7811,
        title: 'lyra-member',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(7801, undefined, { timeout: 15_000 })
    const member = iterationBoard.storyRow(container, 7811)
    expect(member).toBeTruthy()
    const memberGrip = iterationBoard.grip(member)
    expect(memberGrip).toBeTruthy()
    expect(member.getAttribute('data-dragging')).toBeNull()

    const drag = newDragGesture()
    drag.start(memberGrip)
    expect(member.getAttribute('data-dragging')).toBe('true')

    drag.end(memberGrip)
    expect(member.getAttribute('data-dragging')).toBeNull()
  }, 20_000)
})
