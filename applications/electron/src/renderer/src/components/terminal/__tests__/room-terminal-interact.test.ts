import { describe, test, expect, beforeEach } from 'vitest'

import {
  resetStateForTest,
  captureSentFramesForTest,
  sendPtyInput,
  sendChat,
  encodeKeystroke,
  decodePtyData
} from '@state'

describe('interact — raw keystrokes reach the PTY stdin plane', () => {
  beforeEach(() => resetStateForTest())

  test('a control-key encodes as its raw byte, not a line-buffered string', () => {
    expect(encodeKeystroke('\x03')).toBe('Aw==')
    expect(decodePtyData(encodeKeystroke('\x03'))).toEqual(new Uint8Array([0x03]))
    expect(decodePtyData(encodeKeystroke('y'))).toEqual(new Uint8Array([0x79]))
  })

  test('an arrow-key escape sequence passes through as its raw bytes', () => {
    expect(decodePtyData(encodeKeystroke('\x1b[A'))).toEqual(new Uint8Array([0x1b, 0x5b, 0x41]))
  })

  test('typing into the terminal sends a raw pty_input frame carrying the keystroke bytes', () => {
    const frames = captureSentFramesForTest()
    sendPtyInput('dev', '\x03')
    expect(frames).toEqual([{ type: 'pty_input', terminal_id: 'dev', data: 'Aw==' }])
  })

  test('mode-boundary: chat-submit is the line-buffered send plane, never pty_input', () => {
    const frames = captureSentFramesForTest()
    sendChat('hello team')
    expect(frames).toEqual([{ type: 'send', text: 'hello team' }])
    expect(frames.some((f) => f.type === 'pty_input')).toBe(false)
  })
})
