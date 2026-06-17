import { describe, expect, test } from 'vitest'
import {
  errorDecoder,
  pongDecoder,
  ptyExitDecoder,
  ptyOutputDecoder,
  welcomeDecoder
} from './wsDecoders'

describe('welcomeDecoder', () => {
  test('decodes a welcome frame', () => {
    const result = welcomeDecoder.validate({ type: 'welcome', session_id: 'abc' })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.session_id).toBe('abc')
  })

  test('rejects a non-welcome frame (wrong type) — the chain falls through to the next contract', () => {
    expect(welcomeDecoder.validate({ type: 'pong' }).type).toBe('error')
  })

  test('rejects a malformed welcome (missing session_id) — fails loud, not silently cast', () => {
    expect(welcomeDecoder.validate({ type: 'welcome' }).type).toBe('error')
  })
})

describe('errorDecoder', () => {
  test('decodes an error frame', () => {
    expect(errorDecoder.validate({ type: 'error', reason: 'boom' }).type).toBe('ok')
  })

  test('rejects a non-error frame', () => {
    expect(errorDecoder.validate({ type: 'welcome', session_id: 'x' }).type).toBe('error')
  })
})

describe('pongDecoder', () => {
  test('decodes a pong frame', () => {
    expect(pongDecoder.validate({ type: 'pong' }).type).toBe('ok')
  })

  test('rejects a non-pong frame', () => {
    expect(pongDecoder.validate({ type: 'error', reason: 'x' }).type).toBe('error')
  })
})

describe('ptyOutputDecoder', () => {
  test('decodes a pty_output frame', () => {
    const result = ptyOutputDecoder.validate({ type: 'pty_output', terminal_id: 't1', data: 'hi' })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.terminal_id).toBe('t1')
  })

  test('rejects a pty_output missing data', () => {
    expect(ptyOutputDecoder.validate({ type: 'pty_output', terminal_id: 't1' }).type).toBe('error')
  })
})

describe('ptyExitDecoder', () => {
  test('decodes a pty_exit frame', () => {
    expect(ptyExitDecoder.validate({ type: 'pty_exit', terminal_id: 't1' }).type).toBe('ok')
  })

  test('rejects a non-pty_exit frame', () => {
    expect(ptyExitDecoder.validate({ type: 'pty_output', terminal_id: 't1', data: 'x' }).type).toBe(
      'error'
    )
  })
})
