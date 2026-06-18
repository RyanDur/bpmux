import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from '../board/iterationBoard.page'

describe('#1339 Sc1 user-axis — drop on displaced source row still posts label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('drop fires on the source <li> (eagerReorder-displaced slot); mutation still uses hoveredNumber as target', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 9501,
        title: 'Iteration DisplacedDropTheme',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 9511,
        title: 'displaced-alpha',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#9501.100']
      },
      {
        number: 9512,
        title: 'displaced-bravo',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#9501.200']
      },
      {
        number: 9513,
        title: 'displaced-charlie',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#9501.300']
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(9501, undefined, { timeout: 15_000 })
    const source = iterationBoard.storyRow(container, 9513)
    const sourceGrip = iterationBoard.grip(source)
    const target = iterationBoard.storyRow(container, 9511)
    expect(source).toBeTruthy()
    expect(sourceGrip).toBeTruthy()
    expect(target).toBeTruthy()

    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(target)
    drag.drop(source)

    expect(source.getAttribute('data-dragging')).not.toBe('true')

    await waitFor(
      async () => {
        const boardRes = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const boardBody = (await boardRes.json()) as {
          stories: {
            number: number
            iteration_membership: { iteration_number: number; position: number } | null
          }[]
        }
        const moved = boardBody.stories.find((s) => s.number === 9513)
        expect(moved?.iteration_membership).toEqual({ iteration_number: 9501, position: 50 })
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
