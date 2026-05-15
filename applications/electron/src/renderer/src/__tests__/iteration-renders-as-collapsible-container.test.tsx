import { afterEach, describe, expect, inject, it } from 'vitest'
import { fireEvent, waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1297 — iteration renders as a collapsible container in the icebox', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'iteration story renders as a <details> container with summary + body (sister to Epic.tsx)',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7101,
          title: 'Iteration Cassiopeia',
          story_type: 'iteration',
          initial_state: 'icebox'
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7101', undefined, {
        timeout: 15_000
      })
      expect(container.tagName.toLowerCase()).toBe('details')
      const summary = container.querySelector('summary')
      expect(summary).toBeTruthy()
      expect(summary?.textContent).toContain('Cassiopeia')
    },
    25_000
  )

  it(
    'iteration collapse-state is per-user-local and persists across remount (sister to useEpicCollapse)',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7102,
          title: 'Iteration Pegasus',
          story_type: 'iteration',
          initial_state: 'icebox'
        }
      ])

      rendered = renderApp()

      const container = (await rendered.findByTestId('board-iteration-7102', undefined, {
        timeout: 15_000
      })) as HTMLDetailsElement

      const initiallyOpen = container.open
      const summary = container.querySelector('summary')
      expect(summary).toBeTruthy()
      fireEvent.click(summary as HTMLElement)
      await waitFor(() => expect(container.open).toBe(!initiallyOpen))

      rendered.cleanup()
      rendered = renderApp()
      const remounted = (await rendered.findByTestId('board-iteration-7102', undefined, {
        timeout: 15_000
      })) as HTMLDetailsElement
      expect(remounted.open).toBe(!initiallyOpen)
    },
    25_000
  )

  it(
    'an Unscheduled container exists when there are loose icebox stories without iteration-membership',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      await seedStories(apiBase, authToken, [
        {
          number: 7201,
          title: 'a loose icebox story',
          story_type: 'feature',
          initial_state: 'icebox'
        }
      ])

      rendered = renderApp()

      const unscheduled = await rendered.findByTestId('board-iteration-unscheduled', undefined, {
        timeout: 15_000
      })
      expect(unscheduled.tagName.toLowerCase()).toBe('details')
      expect(unscheduled.textContent).toContain('Unscheduled')
      expect(unscheduled.getAttribute('data-iteration-kind')).toBe('unscheduled')
    },
    25_000
  )
})
