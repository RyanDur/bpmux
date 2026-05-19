import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

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
        },
        {
          number: 7100,
          title: 'cassiopeia-member',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-Cassiopeia.100']
        }
      ])

      rendered = renderApp()

      const container = await rendered.findByTestId('board-iteration-7101', undefined, {
        timeout: 15_000
      })
      expect(container.tagName.toLowerCase()).toBe('details')
      const summary = within(container).getByTestId('board-iteration-summary')
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
          title: 'Iteration RemountCollapseScope',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 7103,
          title: 'remount-collapse-scope-member',
          story_type: 'feature',
          initial_state: 'icebox',
          labels: ['iteration-RemountCollapseScope.100']
        }
      ])

      rendered = renderApp()

      const container = (await rendered.findByTestId('board-iteration-7102', undefined, {
        timeout: 15_000
      })) as HTMLDetailsElement

      const initiallyOpen = container.open
      const summary = within(container).getByTestId('board-iteration-summary')
      expect(summary).toBeTruthy()
      const user = userEvent.setup()
      await user.click(summary as HTMLElement)
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
