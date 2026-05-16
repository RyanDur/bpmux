import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 — drag-within-iteration visual no snap-back (optimistic UI)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'after drop, the rendered member order reflects the new position without waiting for backend state-delta — falsifier for the snap-back bug Ryan flagged at 04:18 UTC',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7701,
          title: 'Iteration OptimisticVisualTheme',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 7711,
          title: 'opt-alpha',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-OptimisticVisualTheme.100']
        },
        {
          number: 7712,
          title: 'opt-bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-OptimisticVisualTheme.200']
        },
        {
          number: 7713,
          title: 'opt-charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-OptimisticVisualTheme.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7701', undefined, {
        timeout: 15_000
      })
      const queryOrder = (): number[] =>
        Array.from(container.querySelectorAll('[data-iteration-member]')).map((el) =>
          Number(el.getAttribute('data-iteration-member'))
        )
      const orderBefore = queryOrder()
      expect(orderBefore).toEqual([7711, 7712, 7713])

      const source = container.querySelector('[data-iteration-member="7713"]') as HTMLLIElement
      const target = container.querySelector('[data-iteration-member="7711"]') as HTMLLIElement
      const dataStore: Record<string, string> = {}
      const dt: Pick<DataTransfer, 'setData' | 'getData' | 'types' | 'effectAllowed' | 'dropEffect'> = {
        setData: (type: string, value: string) => {
          dataStore[type] = value
        },
        getData: (type: string) => dataStore[type] ?? '',
        types: [] as unknown as readonly string[],
        effectAllowed: 'move',
        dropEffect: 'move'
      }
      fireEvent.dragStart(source, { dataTransfer: dt })
      Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
      fireEvent.dragOver(target, { dataTransfer: dt })
      fireEvent.drop(target, { dataTransfer: dt })
      fireEvent.dragEnd(source, { dataTransfer: dt })

      await waitFor(
        () => {
          const orderAfter = queryOrder()
          expect(orderAfter[0]).toBe(7713)
        },
        { timeout: 2_000, interval: 50 }
      )
    },
    20_000
  )
})
