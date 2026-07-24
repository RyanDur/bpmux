import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('Sc1 user-axis — drop on displaced source row still posts label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('drop fires on the source <li> (eagerReorder-displaced slot); mutation still uses hoveredId as target', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 9501,
        title: 'Iteration DisplacedDrop',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[9511,9512,9513]} -->'
      },
      {
        id: 9511,
        title: 'displaced-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 9512,
        title: 'displaced-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 9513,
        title: 'displaced-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
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
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { id: number; stories: number[] }[]
        }
        const displacedDrop = iterBody.iterations.find((i) => i.id === 9501)
        expect(displacedDrop?.stories).toEqual([9513, 9511, 9512])
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
