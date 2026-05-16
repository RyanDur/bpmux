import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 Sc1 user-axis — drop on displaced source row still posts label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'drop fires on the source <li> (eagerReorder-displaced slot); mutation still uses hoveredNumber as target',
    async () => {
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
          labels: ['iteration-DisplacedDropTheme.100']
        },
        {
          number: 9512,
          title: 'displaced-bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-DisplacedDropTheme.200']
        },
        {
          number: 9513,
          title: 'displaced-charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-DisplacedDropTheme.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-9501', undefined, {
        timeout: 15_000
      })
      const source = container.querySelector('[data-iteration-member="9513"]') as HTMLLIElement
      const sourceGrip = source.querySelector('[data-test="story-grip"]') as HTMLElement
      const target = container.querySelector('[data-iteration-member="9511"]') as HTMLLIElement
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
      fireEvent.drop(source, { dataTransfer: dt })

      expect(source.getAttribute('data-dragging')).not.toBe('true')

      await waitFor(
        async () => {
          const boardRes = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${authToken}` }
          })
          const boardBody = (await boardRes.json()) as {
            stories: { number: number; labels: string[] }[]
          }
          const moved = boardBody.stories.find((s) => s.number === 9513)
          expect(moved?.labels).toContain('iteration-DisplacedDropTheme.50')
          expect(moved?.labels).not.toContain('iteration-DisplacedDropTheme.300')
        },
        { timeout: 10_000, interval: 200 }
      )
    },
    30_000
  )
})
