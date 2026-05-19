import { afterEach, describe, expect, inject, it } from 'vitest'
import { screen } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../test/fixtures/seedMessages'

const GROUPED_SEED: SeedMessage[] = [
  {
    from: 'pm:claude-code',
    text: 'GROUPED_FIRST_MARKER_BPF18',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'GROUPED_SECOND_MARKER_BPF18',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

const NONGROUPED_SEED: SeedMessage[] = [
  {
    from: 'pm:claude-code',
    text: 'NONGROUPED_FIRST_MARKER_BPF18',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'pm:claude-code',
    text: 'NONGROUPED_SECOND_MARKER_BPF18',
    ts: '2026-04-22T16:06:00Z',
    message_type: 'chat'
  }
]

describe('human-side E2E #476 — read-class — message grouping 5-min threshold', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'two same-author chat messages within 5min collapse the second into a grouped row',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, GROUPED_SEED)

      rendered = renderApp()

      await rendered.findByText(/GROUPED_FIRST_MARKER_BPF18/, undefined, { timeout: 15_000 })
      await rendered.findByText(/GROUPED_SECOND_MARKER_BPF18/, undefined, { timeout: 15_000 })

      const list = screen.getByTestId('message-list')

      const firstRows = list.querySelectorAll('.message-row--first')
      const groupedRows = list.querySelectorAll('.message-row--grouped')
      expect(firstRows.length).toBe(1)
      expect(groupedRows.length).toBe(1)
    },
    20_000
  )

  it(
    'two same-author chat messages 6min apart render as two first-rows, not grouped',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`
      await resetMessages(apiBase, authToken)
      await seedMessages(apiBase, authToken, NONGROUPED_SEED)

      rendered = renderApp()

      await rendered.findByText(/NONGROUPED_SECOND_MARKER_BPF18/, undefined, { timeout: 15_000 })

      const list = screen.getByTestId('message-list')

      const firstRows = list.querySelectorAll('.message-row--first')
      const groupedRows = list.querySelectorAll('.message-row--grouped')
      expect(firstRows.length).toBe(2)
      expect(groupedRows.length).toBe(0)
    },
    20_000
  )
})
