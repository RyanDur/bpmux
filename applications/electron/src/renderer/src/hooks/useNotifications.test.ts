import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import type { ChatMessage } from '../transport/types'

const { liveHandlers } = vi.hoisted(() => ({
  liveHandlers: [] as ((m: ChatMessage) => void)[]
}))

vi.mock('../transport/wsService', () => ({
  RENDERER_IDENTITY: 'ryan',
  subscribeWsLiveMessages: (h: (m: ChatMessage) => void) => {
    liveHandlers.push(h)
    return () => {
      const i = liveHandlers.indexOf(h)
      if (i >= 0) liveHandlers.splice(i, 1)
    }
  }
}))

import { isRelevant, useNotifications } from './useNotifications'
import { RENDERER_IDENTITY } from '../transport/session'

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
    expect(isRelevant(baseMsg({ from: 'pm:claude-code' }), SELF)).toBe(false)
  })

  it('AC2: broadcast with explicit to=["all"] → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['all'] }), SELF)).toBe(false)
  })

  it('AC3: DM with to=[self] → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['dev'] }), SELF)).toBe(true)
  })

  it('AC3: group DM with to including self + others → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'ryan', to: ['dev', 'designer'] }), SELF)).toBe(true)
  })

  it('AC3: DM with to=[other] (not self) → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', to: ['designer'] }), SELF)).toBe(false)
  })

  it('AC4: @mention with addressee=self → relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', addressee: 'dev' }), SELF)).toBe(true)
  })

  it('AC4: @mention with addressee=other → not relevant', () => {
    expect(isRelevant(baseMsg({ from: 'pm:claude-code', addressee: 'designer' }), SELF)).toBe(false)
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
    expect(
      isRelevant(baseMsg({ from: 'pm', addressee: 'dev', messageType: 'activity' }), SELF)
    ).toBe(false)
  })

  it('AC6: system messageType → not relevant even when DM-targeted', () => {
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

describe('#1537 useNotifications — notifies per live-message append (no length-cursor)', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    liveHandlers.length = 0
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
    for (const h of liveHandlers) h(msg)
  }

  it('subscribes at mount and emits NO notification before any live message arrives', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    const ctor = installNotificationSpy()
    renderHook(() => useNotifications())
    expect(liveHandlers.length).toBe(1)
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
