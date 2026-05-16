import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1339 Scenario 4 — grip-handle is the sole drag-initiator', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'renders a grip-handle on draggable rows; clicking the row body opens drill-in, clicking the grip does not',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 8801,
          title: 'Iteration GripTest',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 8811,
          title: 'draggable member',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-GripTest.100']
        }
      ])

      rendered = renderApp()

      const row = await rendered.findByTestId('board-story-8811', undefined, {
        timeout: 15_000
      })
      expect(row.tagName.toLowerCase()).toBe('li')
      expect(row.getAttribute('draggable')).not.toBe('true')

      const grip = row.querySelector('[data-test="story-grip"]') as HTMLElement
      expect(grip).toBeTruthy()
      expect(grip.getAttribute('draggable')).toBe('true')
      expect(grip.getAttribute('aria-label')).toMatch(/drag.*8811/i)

      expect(rendered.queryByTestId('board-drill-in')).toBeNull()

      fireEvent.click(grip)
      expect(rendered.queryByTestId('board-drill-in')).toBeNull()

      const button = row.querySelector('.board-story-button') as HTMLElement
      expect(button).toBeTruthy()
      fireEvent.click(button)
      await waitFor(
        () => {
          expect(rendered!.queryByTestId('board-drill-in')).toBeTruthy()
        },
        { timeout: 5_000 }
      )
    },
    30_000
  )

  it('omits the grip on non-draggable rows (closed-iteration members)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        number: 8802,
        title: 'Iteration ClosedGripTest',
        story_type: 'iteration',
        initial_state: 'icebox',
        is_closed: true
      },
      {
        number: 8812,
        title: 'member of closed iteration',
        story_type: 'feature',
        initial_state: 'icebox',
        labels: ['iteration-ClosedGripTest.100']
      }
    ])

    rendered = renderApp()

    const row = await rendered.findByTestId('board-story-8812', undefined, {
      timeout: 15_000
    })
    const grip = row.querySelector('[data-test="story-grip"]')
    expect(grip).toBeNull()
  }, 30_000)
})
