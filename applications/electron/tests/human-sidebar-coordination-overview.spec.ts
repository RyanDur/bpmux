import { test, expect } from '@playwright/test'
import { launchWithMockGh, waitForMessagesLoaded } from './helpers/launch'

const secAgo = (s: number): string => new Date(Date.now() - s * 1000).toISOString()

test.describe('human-side E2E #1605 — coordination overview: active pairs discriminate from idle', () => {
  test('@-addressed coordination renders active pairs (dot + top) DISTINCT from an idle pair (dimmed + sunk)', async () => {
    const { app, win, mockGh } = await launchWithMockGh({
      seedMessages: [
        {
          from: 'dev:claude-code',
          text: 'dev coordinates with pm on the build',
          ts: secAgo(120),
          addressee: 'pm',
          message_type: 'chat'
        },
        {
          from: 'designer:claude-code',
          text: 'designer coordinates with dev on the render',
          ts: secAgo(20),
          addressee: 'dev',
          message_type: 'chat'
        }
      ],
      seedRoster: ['pm:claude-code', 'dev:claude-code', 'designer:claude-code']
    })
    try {
      const consoleErrors: string[] = []
      const pageErrors: Error[] = []
      win.on('console', (m) => {
        if (m.type() === 'error') consoleErrors.push(m.text())
      })
      win.on('pageerror', (e) => pageErrors.push(e))

      await expect(win.locator('[data-test="sidebar"]')).toBeVisible()
      await waitForMessagesLoaded(win, 'dev coordinates with pm on the build')

      consoleErrors.length = 0
      pageErrors.length = 0

      const devPm = win.locator('[data-test="sidebar-group-dev+pm"]')
      const designerDev = win.locator('[data-test="sidebar-group-designer+dev"]')
      const designerPm = win.locator('[data-test="sidebar-group-designer+pm"]')

      await expect(devPm).toHaveAttribute('data-coordination', 'active')
      await expect(designerDev).toHaveAttribute('data-coordination', 'active')
      await expect(designerPm).toHaveAttribute('data-coordination', 'idle')

      await expect(win.locator('[data-test="coordination-dot-dev+pm"]')).toBeVisible()
      await expect(win.locator('[data-test="coordination-dot-designer+dev"]')).toBeVisible()
      await expect(win.locator('[data-test="coordination-dot-designer+pm"]')).toHaveCount(0)

      await expect(designerPm).toHaveClass(/sidebar-item--idle/)
      await expect(devPm).not.toHaveClass(/sidebar-item--idle/)

      const order = await win
        .locator('[data-test^="sidebar-group-"]')
        .evaluateAll((els) => els.map((e) => e.getAttribute('data-test')))
      expect(order.indexOf('sidebar-group-designer+dev')).toBeLessThan(
        order.indexOf('sidebar-group-dev+pm')
      )
      expect(order.indexOf('sidebar-group-dev+pm')).toBeLessThan(
        order.indexOf('sidebar-group-designer+pm')
      )

      expect(consoleErrors).toEqual([])
      expect(pageErrors).toEqual([])
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
