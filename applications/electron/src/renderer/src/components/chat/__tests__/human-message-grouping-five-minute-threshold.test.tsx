import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../../../test/fixtures/seedMessages'
import { humanMessage } from './humanMessage.page'

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

describe('message grouping 5-min threshold', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('two same-author chat messages within 5min collapse the second into a grouped row', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, GROUPED_SEED)

    rendered = renderApp()

    await humanMessage.waitForText(/GROUPED_FIRST_MARKER_BPF18/)
    await humanMessage.waitForText(/GROUPED_SECOND_MARKER_BPF18/)

    expect(humanMessage.firstRowCount()).toBe(1)
    expect(humanMessage.groupedRowCount()).toBe(1)
  }, 20_000)

  it('two same-author chat messages 6min apart render as two first-rows, not grouped', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, NONGROUPED_SEED)

    rendered = renderApp()

    await humanMessage.waitForText(/NONGROUPED_SECOND_MARKER_BPF18/)

    expect(humanMessage.firstRowCount()).toBe(2)
    expect(humanMessage.groupedRowCount()).toBe(0)
  }, 20_000)
})
