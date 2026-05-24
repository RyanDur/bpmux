import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { isRelevant, useNotifications } from './useNotifications'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import type { ChatMessage } from '../transport/types'

const SELF = 'dev'

const baseMsg = (overrides: Partial<ChatMessage>): ChatMessage => ({
  id: 'msg-1',
  from: 'pm:claude-code',
  text: 'hello',
  ts: '2026-05-05T03:00:00.000Z',
  to: null,
  addressee: null,
  messageType: null,
  ...overrides
})

describe('#382 isRelevant — notification gating matrix', () => {
  it('AC2: broadcast (no addressee, no to) → not relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('AC2: broadcast with explicit to=["all"] → not relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', to: ['all'] })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('AC3: DM with to=[self] → relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', to: ['dev'] })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('AC3: group DM with to including self + others → relevant', () => {
    const msg = baseMsg({ from: 'ryan', to: ['dev', 'designer'] })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('AC3: DM with to=[other] (not self) → not relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', to: ['designer'] })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('AC4: @mention with addressee=self → relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', addressee: 'dev' })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('AC4: @mention with addressee=other → not relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', addressee: 'designer' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('AC5: accept_story signal (addressee=dev, viewer=dev) → relevant', () => {
    const msg = baseMsg({
      from: 'ryan:electron',
      addressee: 'dev',
      text: '#382 accepted by ryan. pick the next story from the ready queue'
    })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('AC5: reject_story signal (addressee=dev, to=[dev]) → relevant', () => {
    const msg = baseMsg({
      from: 'ryan:electron',
      addressee: 'dev',
      to: ['dev'],
      text: '#382 rejected: needs rework. fix and re-deliver'
    })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('AC6: activity messageType → not relevant even when @mentioned', () => {
    const msg = baseMsg({ from: 'pm', addressee: 'dev', messageType: 'activity' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('AC6: system messageType → not relevant even when DM-targeted', () => {
    const msg = baseMsg({ from: 'bpmux', to: ['dev'], messageType: 'system' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('self-from filter: viewer self-message → not relevant', () => {
    const msg = baseMsg({ from: 'dev:claude-code', addressee: 'dev' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('self-from filter respects short-name extraction (strips :suffix)', () => {
    const msg = baseMsg({ from: 'dev', addressee: 'dev' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })

  it('empty to + null addressee + chat messageType → broadcast → not relevant', () => {
    const msg = baseMsg({
      from: 'pm:claude-code',
      to: [],
      addressee: null,
      messageType: 'chat'
    })
    expect(isRelevant(msg, SELF)).toBe(false)
  })
})

describe('#1499 useNotifications — first-run suppression without firstRunRef', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  const relevant = (id: string): ChatMessage =>
    baseMsg({ id, from: 'pm:claude-code', addressee: RENDERER_IDENTITY })

  const installNotificationSpy = (): ReturnType<typeof vi.fn> => {
    const ctor = vi.fn()
    class FakeNotification {
      onclick: (() => void) | null = null
      constructor(title: string, opts?: NotificationOptions) {
        ctor(title, opts)
      }
      close = vi.fn()
      static permission = 'granted'
      static requestPermission = vi.fn()
    }
    vi.stubGlobal('Notification', FakeNotification as unknown as typeof Notification)
    return ctor
  }

  it('emits NO notification on first run even when a relevant message is already present', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    renderHook(({ messages }) => useNotifications(messages), {
      initialProps: { messages: [relevant('seed-1')] }
    })
    expect(ctor).not.toHaveBeenCalled()
  })

  it('emits a notification when a new relevant message arrives after the first run', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    const { rerender } = renderHook(({ messages }) => useNotifications(messages), {
      initialProps: { messages: [relevant('seed-1')] }
    })
    rerender({ messages: [relevant('seed-1'), relevant('fresh-2')] })
    expect(ctor).toHaveBeenCalledTimes(1)
  })
})
