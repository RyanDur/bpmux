import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 Scenario 5 — closed iteration rejects inbound drag with visual feedback', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('dragging an open-iteration member onto a closed iteration sets dropEffect=none and does not mutate labels', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8501,
        title: 'Iteration Carina',
        story_type: 'iteration',
        initial_state: 'icebox'
      },
      {
        number: 8502,
        title: 'Iteration Eridanus',
        story_type: 'iteration',
        initial_state: 'icebox',
        is_closed: true
      },
      {
        number: 8511,
        title: 'carina-member',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['parent-iteration-#8501.100']
      }
    ])

    rendered = renderApp()

    const carina = await rendered.findByTestId('board-iteration-8501', undefined, {
      timeout: 15_000
    })
    const eridanus = await rendered.findByTestId('board-iteration-8502', undefined, {
      timeout: 15_000
    })
    expect(eridanus.getAttribute('data-closed')).toBe('true')

    const source = carina.querySelector('[data-iteration-member="8511"]') as HTMLLIElement
    expect(source).toBeTruthy()
    const sourceGrip = source.querySelector('[data-test="story-grip"]') as HTMLElement
    expect(sourceGrip).toBeTruthy()

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
    fireEvent.dragStart(sourceGrip, { dataTransfer: dt })
    Object.defineProperty(dt, 'types', { value: Object.keys(dataStore) })
    fireEvent.dragOver(eridanus, { dataTransfer: dt })
    expect(dt.dropEffect).toBe('none')
    expect(eridanus.getAttribute('data-drag-rejected')).toBe('true')

    fireEvent.drop(eridanus, { dataTransfer: dt })

    const boardRes = await fetch(`${apiBase}/api/board`, {
      headers: { Authorization: `Bearer ${authToken}` }
    })
    const boardBody = (await boardRes.json()) as {
      stories: {
        number: number
        iteration_membership: { iteration_number: number; position: number } | null
      }[]
    }
    const member = boardBody.stories.find((s) => s.number === 8511)
    expect(member?.iteration_membership).toEqual({ iteration_number: 8501, position: 100 })
  }, 25_000)
})
