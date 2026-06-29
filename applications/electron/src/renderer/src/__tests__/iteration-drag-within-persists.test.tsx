import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from '../board/iterationBoard.page'

describe('#1339 — drag-within-iteration persists via label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dropping member 3 onto member 1 retags member 3 to a position before 100 + state-delta refetch reflects new order', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7601,
        title: 'Iteration Pegasus',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7611,7612,7613]} -->'
      },
      {
        number: 7611,
        title: 'alpha',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        number: 7612,
        title: 'bravo',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        number: 7613,
        title: 'charlie',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const container = await iterationBoard.container(7601, undefined, { timeout: 15_000 })
    expect(iterationBoard.memberOrder(container)).toEqual([7611, 7612, 7613])

    const source = iterationBoard.storyRow(container, 7613)
    const sourceGrip = iterationBoard.grip(source)
    const target = iterationBoard.storyRow(container, 7611)
    expect(source).toBeTruthy()
    expect(target).toBeTruthy()

    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(target)
    drag.drop(target)

    await waitFor(
      async () => {
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { number: number; stories: number[] }[]
        }
        const pegasus = iterBody.iterations.find((i) => i.number === 7601)
        expect(pegasus?.stories).toEqual([7613, 7611, 7612])
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
