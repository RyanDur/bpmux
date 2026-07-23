import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { resetStories, seedStories } from '../../../../test/fixtures/seedMessages'

describe('iteration-itself drag persists via iteration-order label mutation', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dragging Iteration Sirius (order .200) onto Iteration Vega (order .100) orders Sirius before Vega + removes iteration-order.200', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await resetStories(apiBase, authToken)
    await seedStories(apiBase, authToken, [
      {
        id: 8101,
        title: 'Iteration Sirius',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.200']
      },
      {
        id: 8102,
        title: 'Iteration Vega',
        story_type: 'iteration',
        initial_state: 'icebox',
        labels: ['iteration-order.100']
      },
      {
        id: 8110,
        title: 'sirius-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Sirius.100']
      },
      {
        id: 8111,
        title: 'vega-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-Vega.100']
      }
    ])

    rendered = renderApp()

    await screen.findByTestId('board-iteration-8101', undefined, { timeout: 15_000 })
    await screen.findByTestId('board-iteration-8102', undefined, { timeout: 15_000 })

    const siriusGrip = (): HTMLElement => {
      const row = screen.getByTestId('board-iteration-8101').closest('.board-iteration-row')
      const grip = row?.querySelector('[data-test="iteration-grip"]')
      expect(grip).toBeTruthy()
      return grip as HTMLElement
    }
    const vegaRow = (): HTMLElement => screen.getByTestId('board-iteration-8102')

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
    fireEvent.dragStart(siriusGrip(), { dataTransfer: dt })
    Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
    fireEvent.dragOver(vegaRow(), { dataTransfer: dt })
    fireEvent.drop(vegaRow(), { dataTransfer: dt })

    await waitFor(
      async () => {
        const iterRes = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        })
        const iterBody = (await iterRes.json()) as {
          iterations: { id: number; iteration_order: number | null }[]
        }
        const orderOf = (n: number): number | null =>
          iterBody.iterations.find((s) => s.id === n)?.iteration_order ?? null
        const siriusOrder = orderOf(8101)
        const vegaOrder = orderOf(8102)
        expect(siriusOrder).not.toBeNull()
        expect(vegaOrder).not.toBeNull()
        expect(Number(siriusOrder)).toBeLessThan(Number(vegaOrder))
        expect(siriusOrder).not.toBe(200)
      },
      { timeout: 10_000, interval: 200 }
    )
  }, 30_000)
})
