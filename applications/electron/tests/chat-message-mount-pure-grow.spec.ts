import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

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
      await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
      await waitForMessagesLoaded(win, 'mount-animation probe message')
      await win.waitForTimeout(300)

      const transitions = await win.evaluate(
        () =>
          (window as unknown as { __transitions?: TransitionRecord[] }).__transitions ?? []
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
})
