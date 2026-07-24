import { afterEach, describe, expect, inject, it } from 'vitest'
import { waitFor } from '@testing-library/react'

import { renderApp, type RenderAppResult } from '../../../../../../test/fixtures/renderApp'
import {
  resetMessages,
  seedMessages,
  type SeedMessage
} from '../../../../../../test/fixtures/seedMessages'
import { humanMessage } from './humanMessage.page'

const SEED: SeedMessage[] = [
  {
    from: 'pm:claude-code',
    text: 'reactable test message',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  }
]

describe('direct-action — message reaction picker', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it('reaction-add → pick emoji → pill renders on the message (full cycle)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`
    await resetMessages(apiBase, authToken)
    await seedMessages(apiBase, authToken, SEED)

    rendered = renderApp()

    await humanMessage.waitForText(/reactable test message/)

    const addButton = await humanMessage.reactionAddButton()
    expect(humanMessage.reactionPickerOpen(addButton)).toBe(false)

    await humanMessage.openReactionPicker(addButton)
    await waitFor(() => expect(humanMessage.reactionPickerOpen(addButton)).toBe(true))

    expect(await humanMessage.reactionPicker()).toBeTruthy()

    await humanMessage.pickReaction('👍')

    await waitFor(() => expect(humanMessage.reactionPickerCount()).toBe(0))

    await humanMessage.reactionPill('👍')
  }, 20_000)
})
