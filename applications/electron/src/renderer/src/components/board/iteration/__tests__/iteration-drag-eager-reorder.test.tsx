import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('eager-reorder during drag-over (ChosenPicachu pattern)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dragging member-3 over member-1 reorders the visible list to [3, 1, 2] before drop', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 7901,
        title: 'Iteration Orion',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7911,7912,7913]} -->'
      },
      {
        id: 7911,
        title: 'orion-alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7912,
        title: 'orion-bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7913,
        title: 'orion-charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(7901, undefined, { timeout: 15_000 })
    expect(iterationBoard.memberOrder(container)).toEqual([7911, 7912, 7913])

    const source = iterationBoard.storyRow(container, 7913)
    const sourceGrip = iterationBoard.grip(source)
    const target = iterationBoard.storyRow(container, 7911)

    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(target)

    expect(iterationBoard.memberOrder(container)).toEqual([7913, 7911, 7912])
  }, 20_000)
})
