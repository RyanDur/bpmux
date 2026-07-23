import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { ChatMessage } from '../transport/types'

import { isRelevant, useNotifications } from './useNotifications'
import { RENDERER_IDENTITY } from '../state'
import { emitLiveMessage } from '../state/emitter'

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

describe('isRelevant — notification gating matrix', () => {
  it('broadcast (no addressee, no to) → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code' }), SELF)).toBe(false)
  })

  it('broadcast with explicit to=["all"] → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['all'] }), SELF)).toBe(false)
  })

  it('DM with to=[self] → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['dev'] }), SELF)).toBe(true)
  })

  it('group DM with to including self + others → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'owner', to: ['dev', 'designer'] }), SELF)).toBe(true)
  })

  it('DM with to=[other] (not self) → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['designer'] }), SELF)).toBe(false)
  })

  it('@mention with addressee=self → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', addressee: 'dev' }), SELF)).toBe(true)
  })

  it('@mention with addressee=other → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', addressee: 'designer' }), SELF)).toBe(false)
  })

  it('accept_story signal (addressee=dev, viewer=dev) → relevant', () => {
    const msg = baseMsg({
      from: 'owner:electron',
      addressee: 'dev',
      text: '#382 accepted by owner. pick the next story from the ready queue'
    })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('reject_story signal (addressee=dev, to=[dev]) → relevant', () => {
    const msg = baseMsg({
      from: 'owner:electron',
      addressee: 'dev',
      to: ['dev'],
      text: '#382 rejected: needs rework. fix and re-deliver'
    })
    expect(isRelevant(msg, SELF)).toBe(true)
  })

  it('activity messageType → not relevant even when @mentioned', () => {
    expect(
      isRelevant(baseMsg({ from: 'pm', addressee: 'dev', messageType: 'activity' }), SELF)
    ).toBe(false)
  })

  it('system messageType → not relevant even when DM-targeted', () => {
    expect(isRelevant(baseMsg({ from: 'bpmux', to: ['dev'], messageType: 'system' }), SELF)).toBe(
      false
    )
  })

  it('self-from filter: viewer self-message → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'dev:claude-code', addressee: 'dev' }), SELF)).toBe(false)
  })

  it('self-from filter respects short-name extraction (strips :suffix)', () => {
    expect(isRelevant(baseMsg({ from: 'dev', addressee: 'dev' }), SELF)).toBe(false)
  })

  it('empty to + null addressee + chat messageType → broadcast → not relevant', () => {
    const msg = baseMsg({ from: 'pm:claude-code', to: [], addressee: null, messageType: 'chat' })
    expect(isRelevant(msg, SELF)).toBe(false)
  })
})

describe('useNotifications — notifies per live-message append (no length-cursor)', () => {
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

  const fireLive = (msg: ChatMessage): void => {
    emitLiveMessage(msg)
  }

  it('emits NO notification before any live message arrives', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    renderHook(() => useNotifications())
    expect(ctor).not.toHaveBeenCalled()
  })

  it('emits a notification when a relevant live message is appended (viewer inactive)', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    renderHook(() => useNotifications())
    fireLive(relevant('fresh-2'))
    expect(ctor).toHaveBeenCalledTimes(1)
  })

  it('does NOT notify when the viewer is active (focused + visible)', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const ctor = installNotificationSpy()
    renderHook(() => useNotifications())
    fireLive(relevant('fresh-3'))
    expect(ctor).not.toHaveBeenCalled()
  })

  it('does NOT notify for an irrelevant live message (broadcast)', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    renderHook(() => useNotifications())
    fireLive(baseMsg({ id: 'bc', from: 'pm:claude-code', addressee: null, to: null }))
    expect(ctor).not.toHaveBeenCalled()
  })
})
