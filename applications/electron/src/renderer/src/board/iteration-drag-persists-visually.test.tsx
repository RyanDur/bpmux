import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('drag-within-iteration visual no snap-back (optimistic UI)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('after drop, the rendered member order reflects the new position without waiting for backend state-delta — falsifier for the snap-back bug owner flagged', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7701,
        title: 'Iteration OptimisticVisual',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7711,7712,7713]} -->'
      },
      {
        id: 7711,
        title: 'opt-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7712,
        title: 'opt-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7713,
        title: 'opt-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(7701, undefined, { timeout: 15_000 })
    expect(iterationBoard.memberOrder(container)).toEqual([7711, 7712, 7713])

    const source = iterationBoard.storyRow(container, 7713)
    const sourceGrip = iterationBoard.grip(source)
    const target = iterationBoard.storyRow(container, 7711)
    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(target)
    drag.drop(target)
    drag.end(sourceGrip)

    await waitFor(() => expect(iterationBoard.memberOrder(container)[0]).toBe(7713), {
      timeout: 2_000,
      interval: 50
    })
  }, 20_000)
})
