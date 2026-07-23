import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'
import { iterationBoard, newDragGesture } from './iterationBoard.page'

describe('drag-across-iteration retags member with new iteration prefix', () => {
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
        id: 7701,
        title: 'Iteration Vela',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7711]} -->'
      },
      {
        id: 7702,
        title: 'Iteration Cygnus',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[7721]} -->'
      },
      {
        id: 7711,
        title: 'vela-member-one',
        story_type: 'feature',
        initial_state: 'icebox'
      },
      {
        id: 7721,
        title: 'cygnus-member-one',
        story_type: 'feature',
        initial_state: 'icebox'
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
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { id: number; stories: number[] }[]
        }
        const cygnus = iterBody.iterations.find((i) => i.id === 7702)
        const vela = iterBody.iterations.find((i) => i.id === 7701)
        expect(cygnus?.stories).toEqual([7711, 7721])
        expect(vela?.stories).toEqual([])
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
