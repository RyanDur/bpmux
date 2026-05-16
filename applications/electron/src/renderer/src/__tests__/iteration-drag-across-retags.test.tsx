import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 — drag-across-iteration retags member with new iteration prefix', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'dropping a Vela member onto a Cygnus member retags it to iteration-Cygnus + removes iteration-Vela',
    async () => {
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
          labels: ['iteration-Vela.100']
        },
        {
          number: 7721,
          title: 'cygnus-member-one',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Cygnus.100']
        }
      ])

      rendered = renderApp()

      const velaContainer = await rendered.findByTestId('board-iteration-7701', undefined, {
        timeout: 15_000
      })
      const cygnusContainer = await rendered.findByTestId('board-iteration-7702', undefined, {
        timeout: 15_000
      })
      const source = velaContainer.querySelector('[data-iteration-member="7711"]') as HTMLLIElement
      const sourceGrip = source.querySelector('[data-test="story-grip"]') as HTMLElement
      const target = cygnusContainer.querySelector(
        '[data-iteration-member="7721"]'
      ) as HTMLLIElement
      expect(source).toBeTruthy()
      expect(sourceGrip).toBeTruthy()
      expect(target).toBeTruthy()

      const dataStore: Record<string, string> = {}
      const dt: Pick<
        DataTransfer,
        'setData' | 'getData' | 'types' | 'effectAllowed' | 'dropEffect'
      > = {
        setData: (type: string, value: string) => {
          dataStore[type] = value
        },
        getData: (type: string) => dataStore[type] ?? '',
        types: [] as unknown as readonly string[],
        effectAllowed: 'move',
        dropEffect: 'move'
      }
      fireEvent.dragStart(sourceGrip, { dataTransfer: dt })
      Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
      fireEvent.dragOver(target, { dataTransfer: dt })
      fireEvent.drop(target, { dataTransfer: dt })

      await waitFor(
        async () => {
          const boardRes = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${authToken}` }
          })
          const boardBody = (await boardRes.json()) as {
            stories: { number: number; labels: string[] }[]
          }
          const moved = boardBody.stories.find((s) => s.number === 7711)
          expect(moved?.labels).toContain('iteration-Cygnus.50')
          expect(moved?.labels).not.toContain('iteration-Vela.100')
        },
        { timeout: 10_000, interval: 200 }
      )
    },
    30_000
  )
})
