import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 — drag-within-iteration persists via label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'dropping member 3 onto member 1 retags member 3 to a position before 100 + state-delta refetch reflects new order',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7601,
          title: 'Iteration Pegasus',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 7611,
          title: 'alpha',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Pegasus.100']
        },
        {
          number: 7612,
          title: 'bravo',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Pegasus.200']
        },
        {
          number: 7613,
          title: 'charlie',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Pegasus.300']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7601', undefined, {
        timeout: 15_000
      })
      const orderBefore = Array.from(container.querySelectorAll('[data-iteration-member]')).map(
        (el) => Number(el.getAttribute('data-iteration-member'))
      )
      expect(orderBefore).toEqual([7611, 7612, 7613])

      const source = container.querySelector('[data-iteration-member="7613"]') as HTMLLIElement
      const target = container.querySelector('[data-iteration-member="7611"]') as HTMLLIElement
      expect(source).toBeTruthy()
      expect(target).toBeTruthy()

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

      await waitFor(
        async () => {
          const boardRes = await fetch(`${apiBase}/api/board`, {
            headers: { Authorization: `Bearer ${authToken}` }
          })
          const boardBody = (await boardRes.json()) as {
            stories: { number: number; labels: string[] }[]
          }
          const charlie = boardBody.stories.find((s) => s.number === 7613)
          expect(charlie?.labels).toContain('iteration-Pegasus.50')
          expect(charlie?.labels).not.toContain('iteration-Pegasus.300')
        },
        { timeout: 10_000, interval: 200 }
      )
    },
    30_000
  )
})
