import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 step 2d-b — eager-reorder during drag-over (ChosenPicachu pattern)', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'dragging member-3 over member-1 reorders the visible list to [3, 1, 2] before drop',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7901,
          title: 'Iteration Orion',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 7911,
          title: 'orion-alpha',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Orion.100']
        },
        {
          number: 7912,
          title: 'orion-bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Orion.200']
        },
        {
          number: 7913,
          title: 'orion-charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Orion.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7901', undefined, {
        timeout: 15_000
      })
      const orderBefore = Array.from(container.querySelectorAll('[data-iteration-member]')).map(
        (el) => Number(el.getAttribute('data-iteration-member'))
      )
      expect(orderBefore).toEqual([7911, 7912, 7913])

      const source = container.querySelector('[data-iteration-member="7913"]') as HTMLLIElement
      const target = container.querySelector('[data-iteration-member="7911"]') as HTMLLIElement

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
      fireEvent.dragStart(source, { dataTransfer: dt })
      Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
      fireEvent.dragOver(target, { dataTransfer: dt })

      const orderDuringDrag = Array.from(
        container.querySelectorAll('[data-iteration-member]')
      ).map((el) => Number(el.getAttribute('data-iteration-member')))
      expect(orderDuringDrag).toEqual([7913, 7911, 7912])
    },
    20_000
  )
})
