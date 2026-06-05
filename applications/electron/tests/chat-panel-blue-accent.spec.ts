import { test, expect } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const parseRgb = (s: string): number[] => (s.match(/\d+/g) ?? []).slice(0, 3).map(Number)

const relativeLuminance = (rgb: number[]): number => {
  const [r, g, b] = rgb.map((c) => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

const contrastRatio = (a: string, b: string): number => {
  const la = relativeLuminance(parseRgb(a))
  const lb = relativeLuminance(parseRgb(b))
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05)
}

test.describe('#1511 chat-panel blue accent + scope-text retired', () => {
  test('chat-scope text is gone; the blue persists as a chat-panel border-top passing WCAG 1.4.11 (3:1)', async () => {
    const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-1511-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-1511-udd-'))
    const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
    const win = await app.firstWindow()
    try {
      await win.route('**/api/board', async (route) => {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ fetched_at: '2026-06-04T00:00:00Z', stories: [] })
        })
      })

      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      const chat = win.locator('[data-test="chat-region"]')
      await expect(chat).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      await expect(win.locator('.chat-scope')).toHaveCount(0)

      const borderTop = await chat.evaluate((el) => {
        const cs = getComputedStyle(el)
        return { width: cs.borderTopWidth, color: cs.borderTopColor, style: cs.borderTopStyle }
      })
      expect(borderTop.style).toBe('solid')
      expect(parseInt(borderTop.width, 10)).toBeGreaterThanOrEqual(2)
      expect(parseInt(borderTop.width, 10)).toBeLessThanOrEqual(3)

      const tokens = await win.evaluate(() => {
        const probe = document.createElement('span')
        probe.style.color = 'var(--accent-brand)'
        document.body.appendChild(probe)
        const accent = getComputedStyle(probe).color
        probe.style.color = 'var(--bg-base)'
        const base = getComputedStyle(probe).color
        probe.remove()
        return { accent, base }
      })
      expect(borderTop.color).toBe(tokens.accent)
      expect(
        contrastRatio(tokens.accent, tokens.base),
        'the surviving blue accent (non-text graphic) must meet WCAG 1.4.11 3:1 against the panel background'
      ).toBeGreaterThanOrEqual(3)

      const shotPath = join(tmpdir(), 'bpmux-1511-chat-accent.png')
      await win.screenshot({ path: shotPath, fullPage: false })
    } finally {
      await app.close()
    }
  })
})
