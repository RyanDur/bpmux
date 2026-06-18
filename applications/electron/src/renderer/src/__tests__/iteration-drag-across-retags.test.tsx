import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from '../board/iterationBoard.page'

describe('#1339 — drag-across-iteration retags member with new iteration prefix', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dropping a Vela member onto a Cygnus member retags it to iteration-Cygnus + removes iteration-Vela', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 7701,
        title: 'Iteration Vela',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7702,
        title: 'Iteration Cygnus',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 7711,
        title: 'vela-member-one',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7701.100']
      },
      {
        number: 7721,
        title: 'cygnus-member-one',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#7702.100']
      }
    ])

    rendered = renderApp()

    const velaContainer = await iterationBoard.container(7701, undefined, { timeout: 15_000 })
    const cygnusContainer = await iterationBoard.container(7702, undefined, { timeout: 15_000 })
    const source = iterationBoard.memberRow(velaContainer, 7711)
    const sourceGrip = iterationBoard.grip(source)
    const target = iterationBoard.memberRow(cygnusContainer, 7721)
    expect(source).toBeTruthy()
    expect(sourceGrip).toBeTruthy()
    expect(target).toBeTruthy()

    const drag = newDragGesture()
    drag.start(sourceGrip)
    drag.over(target)
    drag.drop(target)

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
        const moved = boardBody.stories.find((s) => s.number === 7711)
        expect(moved?.iteration_membership).toEqual({ iteration_number: 7702, position: 50 })
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
