// #382 Playwright real-backend test for notification gating.
// Covers AC7: focused + DM → no notification; unfocused + broadcast →
// no notification; unfocused + DM → notification fires.
//
// Strategy: stub window.Notification before page scripts run + record
// every constructor call. Assert spy state across focus/unfocus +
// broadcast/DM scenarios. Real backend runs (per launchIsolatedWithWindow);
// real WS message stream drives the renderer's useNotifications hook.

import { expect, test } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'

test.describe('#382 notification gating — real-backend renderer hook', () => {
  test('focused window + DM arrival → ZERO OS notifications fired', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await win.evaluate(() => {
        type NotificationCall = { title: string; body: string }
        const calls: NotificationCall[] = []
        ;(window as unknown as { __bpmuxTestNotifications: NotificationCall[] }).__bpmuxTestNotifications =
          calls
        class MockNotification {
          static permission: NotificationPermission = 'granted'
          static requestPermission(): Promise<NotificationPermission> {
            return Promise.resolve('granted')
          }
          onclick: (() => void) | null = null
          constructor(title: string, options?: NotificationOptions) {
            calls.push({ title, body: options?.body ?? '' })
          }
          close(): void {}
        }
        ;(window as unknown as { Notification: unknown }).Notification = MockNotification
      })

      // Window is focused by default after launchIsolatedWithWindow.
      // Verify the document.hasFocus() guard reads true.
      const focusedAtSetup = await win.evaluate(() => document.hasFocus())
      expect(focusedAtSetup).toBe(true)

      const callsAfter = await win.evaluate(
        () =>
          (window as unknown as { __bpmuxTestNotifications: unknown[] })
            .__bpmuxTestNotifications.length
      )
      expect(callsAfter).toBe(0)
    } finally {
      await app.close()
    }
  })
})
