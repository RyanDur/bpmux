import { describe, expect, test } from 'vitest'
import {
  errorDecoder,
  messageDecoder,
  pongDecoder,
  ptyExitDecoder,
  ptyModeDecoder,
  ptyOutputDecoder,
  welcomeDecoder
} from '../wsDecoders'

describe('welcomeDecoder', () => {
  test('decodes a welcome frame', () => {
    expect(welcomeDecoder.validate({ type: 'welcome' }).type).toBe('ok')
  })

  test('rejects a non-welcome frame (wrong type) — the chain falls through to the next contract', () => {
    expect(welcomeDecoder.validate({ type: 'pong' }).type).toBe('error')
  })
})

describe('errorDecoder', () => {
  test('decodes an error frame', () => {
    expect(errorDecoder.validate({ type: 'error', reason: 'boom' }).type).toBe('ok')
  })

  test('rejects a non-error frame', () => {
    expect(errorDecoder.validate({ type: 'welcome' }).type).toBe('error')
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

describe('ptyModeDecoder', () => {
  test('decodes a pty_mode frame', () => {
    const result = ptyModeDecoder.validate({
      type: 'pty_mode',
      terminal_id: 't1',
      bracketed_paste: true
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.bracketed_paste).toBe(true)
  })

  test('rejects a pty_mode missing bracketed_paste', () => {
    expect(ptyModeDecoder.validate({ type: 'pty_mode', terminal_id: 't1' }).type).toBe('error')
  })
})

describe('messageDecoder', () => {
  test('decodes a plain chat message', () => {
    const result = messageDecoder.validate({
      type: 'message',
      from: 'pm',
      text: 'hi',
      ts: '2026-06-17T00:00:00Z'
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.from).toBe('pm')
  })

  test('decodes a message carrying a known state_delta', () => {
    const result = messageDecoder.validate({
      type: 'message',
      from: 'bpmux',
      text: '',
      ts: 't',
      message_type: 'state-delta',
      state_delta: { domain: 'board_invalidated', reason: 'changed' }
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.state_delta?.domain).toBe('board_invalidated')
  })

  test('tolerates an unknown state_delta domain — decodes to null, not a reject (fork b)', () => {
    const result = messageDecoder.validate({
      type: 'message',
      from: 'bpmux',
      text: '',
      ts: 't',
      message_type: 'state-delta',
      state_delta: { domain: 'story_transition', transition: {} }
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') expect(result.data.state_delta).toBeNull()
  })

  test('decodes the additive delivery id fields when present', () => {
    const result = messageDecoder.validate({
      type: 'message',
      from: 'dev:claude-code',
      text: 'hi',
      ts: '2026-07-09T00:00:00Z',
      sender_id: 'id-dev',
      to_ids: ['id-pm'],
      addressee_ids: ['id-pm']
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') {
      expect(result.data.sender_id).toBe('id-dev')
      expect(result.data.to_ids).toEqual(['id-pm'])
      expect(result.data.addressee_ids).toEqual(['id-pm'])
    }
  })

  test('decodes a legacy message with no delivery id fields to null', () => {
    const result = messageDecoder.validate({
      type: 'message',
      from: 'pm',
      text: 'hi',
      ts: '2026-06-17T00:00:00Z'
    })
    expect(result.type).toBe('ok')
    if (result.type === 'ok') {
      expect(result.data.sender_id ?? null).toBeNull()
      expect(result.data.to_ids ?? null).toBeNull()
      expect(result.data.addressee_ids ?? null).toBeNull()
    }
  })
})
