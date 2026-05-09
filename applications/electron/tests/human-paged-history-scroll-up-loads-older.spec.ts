import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

// #1153 rework — visual-class falsifier per
// `feedback_visual_class_falsifier_needs_visual_assertion`. The
// scrollMath unit test covers the predicate; this spec covers the
// wired path: handleScroll → triggerableLoadOlder → loadOlder fetch
// → loading-pill DOM-shape. Also covers the "no-jank" falsifier:
// fetch is in-flight BEFORE the user reaches the boundary.

const SEED_COUNT = 60

const seedMessages = (count: number): Array<Record<string, unknown>> =>
  Array.from({ length: count }, (_, i) => {
    const minute = String(i % 60).padStart(2, '0')
    const hour = String(10 + Math.floor(i / 60)).padStart(2, '0')
    return {
      from: 'dev:claude-code',
      text: `seed message ${i}`,
      ts: `2026-05-03T${hour}:${minute}:00Z`,
      message_type: 'chat'
    }
  })

test.describe('human-side E2E #1153 — scroll-up triggers paged-history fetch', () => {
  test('scroll near visual top fires loadOlder + loading-pill renders', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: seedMessages(SEED_COUNT)
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await waitForMessagesLoaded(win, 'seed message 59')

      // Intercept the paged-history endpoint so we can deterministically
      // observe loadOlder firing AND control the loading-pill window.
      let pagedFetchCount = 0
      let releasePagedFetch: () => void = () => {}
      const pagedFetchHeld = new Promise<void>((resolve) => {
        releasePagedFetch = resolve
      })
      const olderPage = Array.from({ length: 5 }, (_, i) => ({
        from: 'pm:claude-code',
        text: `older message ${i}`,
        ts: `2026-05-03T05:${String(i).padStart(2, '0')}:00Z`,
        message_type: 'chat'
      }))
      await win.route('**/api/messages?**before=*', async (route) => {
        pagedFetchCount += 1
        await pagedFetchHeld
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(olderPage)
        })
      })

      // Programmatically scroll the message-list to the visual top.
      // Under column-reverse, |scrollTop| grows toward
      // (scrollHeight - clientHeight). Setting scrollTop near that
      // limit puts the user inside the 200px near-top trigger zone.
      const ul = win.locator('[data-test="message-list"]')
      await expect(ul).toBeVisible()
      await ul.evaluate((el) => {
        const target = -(el.scrollHeight - el.clientHeight)
        el.scrollTop = target
        el.dispatchEvent(new Event('scroll'))
      })

      // Falsifier: loading-pill must appear BEFORE the response lands
      // (no-jank — fetch is in flight before user reaches boundary).
      const loadingPill = win.locator('output[data-test="loading-older-pill"]')
      await expect(loadingPill).toBeVisible({ timeout: 2000 })

      // loadOlder fired exactly the network call we expected.
      expect(pagedFetchCount).toBeGreaterThan(0)

      // Release the held fetch so the merge path runs.
      releasePagedFetch()

      // Pill clears once the response lands.
      await expect(loadingPill).toHaveCount(0, { timeout: 5000 })

      // Older messages prepended into the rendered list.
      await expect(ul).toContainText('older message 0')

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
