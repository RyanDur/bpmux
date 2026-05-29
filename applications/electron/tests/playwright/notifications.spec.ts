// #382 Playwright real-backend test for notification gating.
// Covers AC7 — focused/unfocused × broadcast/DM matrix:
// - test 1: focused + DM → ZERO notifications fired
// - test 2: unfocused + DM → notification fires
// - test 3: unfocused + broadcast → ZERO notifications fired
//
// Strategy: stub window.Notification before page scripts run + record
// every constructor call. Stub document.hasFocus → false to simulate
// unfocused for tests 2 + 3. Inject messages via POST /api/messages on
// the running bpmux backend so the WS round-trip drives the renderer's
// useNotifications hook end-to-end. Per
// feedback_falsifier_load_bearing_test_at_e2e_layer: each falsifier
// names a SCENARIO; tests drive THAT scenario at e2e layer.

import { expect, test, type Page } from '@playwright/test'
import { launchIsolatedWithWindow } from '../helpers/launch'

interface NotificationCall {
  title: string
  body: string
}

declare global {
  interface Window {
    __bpmuxTestNotifications?: NotificationCall[]
    bpmux?: { apiBase: string; authToken: string; wsUrl: string }
  }
}

const installNotificationStub = `
  const calls = window.__bpmuxTestNotifications = [];
  class MockNotification {
    static permission = 'granted';
    static requestPermission() { return Promise.resolve('granted'); }
    onclick = null;
    constructor(title, options) { calls.push({ title, body: (options && options.body) || '' }); }
    close() {}
  }
  window.Notification = MockNotification;
`

const stubFocusFalse = `
  Object.defineProperty(document, 'hasFocus', { value: () => false, configurable: true });
  Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
`

const awaitLiveWsChannel = async (win: Page, apiBase: string, authToken: string): Promise<void> => {
  await expect(async () => {
    const probe = `ws-live-probe-${Math.random().toString(36).slice(2)}`
    await win.evaluate(
      async ([base, token, text]) => {
        await fetch(`${base}/api/messages`, {
          method: 'POST',
          headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
          body: JSON.stringify({ from: 'pm', text })
        })
      },
      [apiBase, authToken, probe] as const
    )
    await expect(win.locator('[data-test="message-list"]')).toContainText(probe, {
      timeout: 1500
    })
  }).toPass({ timeout: 20000 })
}

test.describe('#382 notification gating — real-backend matrix', () => {
  test('focused + DM arrival → ZERO OS notifications fired (AC1)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await win.addInitScript(installNotificationStub)
      await win.reload()
      const focusedAtSetup = await win.evaluate(() => document.hasFocus())
      expect(focusedAtSetup).toBe(true)
      const bridge = await win.evaluate(() => window.bpmux)
      expect(bridge).toBeDefined()
      await win.evaluate(
        async ([apiBase, token]) => {
          await fetch(`${apiBase}/api/messages`, {
            method: 'POST',
            headers: {
              'content-type': 'application/json',
              authorization: `Bearer ${token}`
            },
            body: JSON.stringify({
              from: 'pm',
              text: 'focused-DM-target',
              to: ['ryan']
            })
          })
        },
        [bridge!.apiBase, bridge!.authToken]
      )
      await win.waitForTimeout(800)
      const calls = await win.evaluate(() => window.__bpmuxTestNotifications?.length ?? -1)
      expect(calls).toBe(0)
    } finally {
      await app.close()
    }
  })

  test('unfocused + DM arrival → OS notification fires (AC3)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await win.addInitScript(installNotificationStub)
      await win.addInitScript(stubFocusFalse)
      await win.reload()
      const focused = await win.evaluate(() => document.hasFocus())
      expect(focused).toBe(false)
      const bridge = await win.evaluate(() => window.bpmux)
      expect(bridge).toBeDefined()
      await awaitLiveWsChannel(win, bridge!.apiBase, bridge!.authToken)
      await win.evaluate(
        async ([apiBase, token]) => {
          await fetch(`${apiBase}/api/messages`, {
            method: 'POST',
            headers: {
              'content-type': 'application/json',
              authorization: `Bearer ${token}`
            },
            body: JSON.stringify({
              from: 'pm',
              text: 'unfocused-DM-target',
              to: ['ryan']
            })
          })
        },
        [bridge!.apiBase, bridge!.authToken]
      )
      await expect
        .poll(async () =>
          win.evaluate(() =>
            (window.__bpmuxTestNotifications ?? []).some((c) =>
              c.body.includes('unfocused-DM-target')
            )
          )
        )
        .toBe(true)
    } finally {
      await app.close()
    }
  })

  test('unfocused + broadcast arrival → ZERO OS notifications fired (AC2)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })
    try {
      await win.addInitScript(installNotificationStub)
      await win.addInitScript(stubFocusFalse)
      await win.reload()
      const focused = await win.evaluate(() => document.hasFocus())
      expect(focused).toBe(false)
      const bridge = await win.evaluate(() => window.bpmux)
      expect(bridge).toBeDefined()
      await win.evaluate(
        async ([apiBase, token]) => {
          await fetch(`${apiBase}/api/messages`, {
            method: 'POST',
            headers: {
              'content-type': 'application/json',
              authorization: `Bearer ${token}`
            },
            body: JSON.stringify({
              from: 'pm',
              text: 'unfocused-broadcast-target'
            })
          })
        },
        [bridge!.apiBase, bridge!.authToken]
      )
      await win.waitForTimeout(1500)
      const calls = await win.evaluate(() => window.__bpmuxTestNotifications?.length ?? -1)
      expect(calls).toBe(0)
    } finally {
      await app.close()
    }
  })
})
