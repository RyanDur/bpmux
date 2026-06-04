import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 step 2g-b — iteration-itself drag persists via iteration-order label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dragging Iteration Sirius (order .200) onto Iteration Vega (order .100) sets Sirius iteration-order.50 + removes iteration-order.200', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8101,
        title: 'Iteration Sirius',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.200']
      },
      {
        number: 8102,
        title: 'Iteration Vega',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.100']
      },
      {
        number: 8110,
        title: 'sirius-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Sirius.100']
      },
      {
        number: 8111,
        title: 'vega-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Vega.100']
      }
    ])

    rendered = renderApp()

    const sirius = await rendered.findByTestId('board-iteration-8101', undefined, {
      timeout: 15_000
    })
    const siriusRow = sirius.closest('.board-iteration-row') as HTMLElement
    const siriusGrip = siriusRow.querySelector('[data-test="iteration-grip"]') as HTMLElement
    expect(siriusGrip).toBeTruthy()
    const vega = await rendered.findByTestId('board-iteration-8102', undefined, {
      timeout: 15_000
    })

    const dataStore: Record<string, string> = {}
    const dt: Pick<DataTransfer, 'setData' | 'getData' | 'types' | 'effectAllowed' | 'dropEffect'> =
      {
        setData: (type: string, value: string) => {
          dataStore[type] = value
        },
        getData: (type: string) => dataStore[type] ?? '',
        types: [] as unknown as readonly string[],
        effectAllowed: 'move',
        dropEffect: 'move'
      }
    fireEvent.dragStart(siriusGrip, { dataTransfer: dt })
    Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
    fireEvent.dragOver(vega, { dataTransfer: dt })
    fireEvent.drop(vega, { dataTransfer: dt })

    await waitFor(
      async () => {
        const boardRes = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const boardBody = (await boardRes.json()) as {
          stories: { number: number; labels: string[] }[]
        }
        const moved = boardBody.stories.find((s) => s.number === 8101)
        expect(moved?.labels).toContain('iteration-order.50')
        expect(moved?.labels).not.toContain('iteration-order.200')
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
