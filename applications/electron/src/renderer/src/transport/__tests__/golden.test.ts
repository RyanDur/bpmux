import { describe, expect, test } from 'vitest'
import serverChatGolden from '../../../../../../../components/protocol/golden/server-chat-message.json'
import { messageDecoder } from '../wsDecoders'

describe('the type mirror golden — both sides of the wire pin the same checked-in frame', () => {
  test('the renderer decodes the exact frame the Rust serializer produces', () => {
    const result = messageDecoder.validate(serverChatGolden)
    expect(result.type).toBe('ok')
    if (result.type !== 'ok') return
    expect(result.data.from).toBe('dev:claude-code')
    expect(result.data.sender_id).toBe('6b3054e9-a6b4-4e5b-84db-f788ebdec25b')
    expect(result.data.to_ids).toEqual(['pm-alpha'])
    expect(result.data.addressee_ids).toEqual(['pm-alpha'])
    expect(result.data.message_type).toBe('chat')
  })
})
