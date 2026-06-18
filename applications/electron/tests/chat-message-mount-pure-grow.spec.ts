import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

type TransitionRecord = { cls: string; prop: string }

test.describe('#1487 chat-message mount is a pure grid-fr height-grow (no fade)', () => {
  test('the grid-template-rows height-grow PLAYS on mount and the avatar opacity-fade does NOT', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'mount-animation probe message',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['dev:claude-code']
    })
    try {
      await win.addInitScript(() => {
        const store: TransitionRecord[] = []
        ;(window as unknown as { __transitions: TransitionRecord[] }).__transitions = store
        document.addEventListener(
          'transitionrun',
          (e) => {
            const ev = e as TransitionEvent
            const target = ev.target
            const cls = target instanceof Element ? (target.getAttribute('class') ?? '') : ''
            store.push({ cls, prop: ev.propertyName })
          },
          true
        )
      })
      await win.reload()
      await expect(new ChatPage(win).input()).toBeVisible()
      await waitForMessagesLoaded(win, 'mount-animation probe message')
      await win.waitForTimeout(300)

      const transitions = await win.evaluate(
        () => (window as unknown as { __transitions?: TransitionRecord[] }).__transitions ?? []
      )
      const rowGrow = transitions.filter(
        (t) => t.prop === 'grid-template-rows' && t.cls.includes('message-row')
      )
      const avatarFade = transitions.filter(
        (t) => t.prop === 'opacity' && t.cls.includes('message-avatar')
      )

      expect(
        rowGrow.length,
        'the message-row mount must PLAY a grid-template-rows height-grow (transitionrun fired) — not just declare the property; a mis-triggering @starting-style looks right statically but never runs'
      ).toBeGreaterThan(0)
      expect(
        avatarFade.length,
        'the avatar opacity-fade must NOT play on mount (zero opacity transitionrun on .message-avatar) — pure height-grow, no fade layered on'
      ).toBe(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the transform rise PLAYS on a genuinely-new message but NOT on seed-load (cascade-complement)', async () => {
    const { app, win, mockGh, backendPort, authToken } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'seed baseline message',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['dev:claude-code', 'pm:claude-code']
    })
    try {
      await win.addInitScript(() => {
        const store: { text: string }[] = []
        ;(window as unknown as { __riseTransforms: { text: string }[] }).__riseTransforms = store
        document.addEventListener(
          'transitionrun',
          (e) => {
            const ev = e as TransitionEvent
            if (ev.propertyName !== 'transform') return
            const target = ev.target
            if (!(target instanceof HTMLElement) || target.tagName !== 'ARTICLE') return
            const row = target.closest('.message-row')
            if (row === null) return
            store.push({ text: row.textContent ?? '' })
          },
          true
        )
      })
      await win.reload()
      await expect(new ChatPage(win).input()).toBeVisible()
      await waitForMessagesLoaded(win, 'seed baseline message')
      await win.waitForTimeout(300)

      const onSeedLoad = await win.evaluate(
        () =>
          (window as unknown as { __riseTransforms?: { text: string }[] }).__riseTransforms ?? []
      )
      expect(
        onSeedLoad.length,
        'the transform rise must NOT play on seed-load (entering-set empty → no whole-list cascade); a transform transitionrun here means the gate failed'
      ).toBe(0)

      const sent = await fetch(`http://127.0.0.1:${backendPort}/api/messages`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${authToken}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          from: 'pm:claude-code',
          text: 'genuinely-new rise probe',
          ts: '2026-04-22T17:00:00Z',
          message_type: 'chat'
        })
      })
      expect(sent.ok).toBe(true)

      await expect(new ChatPage(win).messageRow('genuinely-new rise probe')).toBeVisible({
        timeout: 10_000
      })
      await win.waitForTimeout(300)

      const afterNew = await win.evaluate(
        () =>
          (window as unknown as { __riseTransforms?: { text: string }[] }).__riseTransforms ?? []
      )
      expect(
        afterNew.some((t) => t.text.includes('genuinely-new rise probe')),
        'the transform rise must PLAY on the genuinely-new message row (transitionrun fired for transform) — proving the @starting-style rise actually runs, not just that the CSS declares it'
      ).toBe(true)
      expect(
        afterNew.some((t) => t.text.includes('seed baseline message')),
        'the rise must NOT play on the prior (seed) row — data-entering is scoped to the new arrival only'
      ).toBe(false)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
