import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { renderApp, type RenderAppResult } from '../../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../../test/fixtures/seedMessages'
import { keyboardReorder } from './keyboardReorder.page'

describe('grip-handle is the sole drag-initiator', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('renders a grip-handle on draggable rows; clicking the row body opens drill-in, clicking the grip does not', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 8801,
        title: 'Iteration GripTest',
        story_type: 'iteration',
        initial_state: 'icebox',
        body: '<!-- bpmux-relationships: {"stories":[8811]} -->'
      },
      {
        id: 8811,
        title: 'draggable member',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const row = await keyboardReorder.storyRow(8811, { timeout: 15_000 })
    expect(row.tagName.toLowerCase()).toBe('li')
    expect(row.getAttribute('draggable')).not.toBe('true')

    const grip = keyboardReorder.grip(row) as HTMLElement
    expect(grip).toBeTruthy()
    expect(grip.getAttribute('draggable')).toBe('true')
    expect(grip.getAttribute('aria-label')).toMatch(/drag.*8811/i)

    expect(keyboardReorder.drillIn()).toBeNull()

    const user = userEvent.setup()
    await user.click(grip)
    expect(keyboardReorder.drillIn()).toBeNull()

    const button = keyboardReorder.navigate(row) as HTMLElement
    expect(button).toBeTruthy()
    await user.click(button)
    await waitFor(
      () => {
        expect(keyboardReorder.drillIn()).toBeTruthy()
      },
      { timeout: 5_000 }
    )
  }, 30_000)

  it('omits the grip on non-draggable rows (closed-iteration members)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    await seedStories(apiBase, authToken, [
      {
        id: 8802,
        title: 'Iteration ClosedGripTest',
        story_type: 'iteration',
        initial_state: 'icebox',
        is_closed: true,
        body: '<!-- bpmux-relationships: {"stories":[8812]} -->'
      },
      {
        id: 8812,
        title: 'member of closed iteration',
        story_type: 'feature',
        initial_state: 'icebox'
      }
    ])

    rendered = renderApp()

    const row = await keyboardReorder.storyRow(8812, { timeout: 15_000 })
    const grip = keyboardReorder.grip(row)
    expect(grip).toBeNull()
  }, 30_000)
})
