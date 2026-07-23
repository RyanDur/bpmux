import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('closed iteration rejects inbound drag with visual feedback', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dragging an open-iteration member onto a closed iteration sets dropEffect=none and does not mutate labels', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 8501,
        title: 'Iteration Carina',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[8511]} -->'
      },
      {
        id: 8502,
        title: 'Iteration Eridanus',
        story_type: 'iteration',
        initial_state: 'icebox',
        is_closed: true,
        body: '<!-- bpmux-relationships: {"stories":[]} -->'
      },
      {
        id: 8511,
        title: 'carina-member',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const carina = await iterationBoard.container(8501, undefined, { timeout: 15_000 })
    const eridanus = await iterationBoard.container(8502, undefined, { timeout: 15_000 })
    expect(eridanus.getAttribute('data-closed')).toBe('true')

    const source = iterationBoard.memberRow(carina, 8511)
    expect(source).toBeTruthy()
    const sourceGrip = iterationBoard.grip(source)
    expect(sourceGrip).toBeTruthy()

    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(eridanus)
    expect(drag.dropEffect()).toBe('none')
    expect(eridanus.getAttribute('data-drag-rejected')).toBe('true')

    drag.drop(eridanus)

    const iterRes = await fetch(`${apiBase}/api/iterations`, {
      headers: { Authorization: `Bearer ${authToken}` }
    })
    const iterBody = (await iterRes.json()) as {
      iterations: { id: number; stories: number[] }[]
    }
    const carinaIteration = iterBody.iterations.find((i) => i.id === 8501)
    const eridanusIteration = iterBody.iterations.find((i) => i.id === 8502)
    expect(carinaIteration?.stories).toEqual([8511])
    expect(eridanusIteration?.stories).toEqual([])
  }, 25_000)
})
