import { test, expect, type Page } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

const STATES = ['connected', 'connecting', 'disconnected'] as const

type StateContrast = { color: string; bg: string; ratio: number }

const measureStateTextContrast = async (
  win: Page,
  theme: 'light' | 'dark'
): Promise<Record<string, StateContrast>> => {
  await win.evaluate((t) => window.localStorage.setItem('bpmux-theme', t), theme)
  await win.reload()
  await expect(win.getByRole('main')).toBeVisible()
  await expect.poll(() => win.evaluate(() => document.documentElement.dataset.theme)).toBe(theme)

  return win.evaluate(
    (states) => {
      const parseRgb = (s: string): number[] =>
        (s.match(/\d+(\.\d+)?/g) ?? []).slice(0, 3).map(Number)
      const relLum = (rgb: number[]): number => {
        const [r, g, b] = rgb.map((c) => {
          const v = c / 255
          return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
        })
        return 0.2126 * r + 0.7152 * g + 0.0722 * b
      }
      const contrast = (a: string, b: string): number => {
        const la = relLum(parseRgb(a))
        const lb = relLum(parseRgb(b))
        return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05)
      }
      const results: Record<string, { color: string; bg: string; ratio: number }> = {}
      for (const state of states) {
        const bar = document.createElement('header')
        bar.className = 'titlebar'
        const app = document.createElement('span')
        app.className = `titlebar-app titlebar-app--${state}`
        const txt = document.createElement('small')
        txt.className = 'titlebar-app-state'
        txt.textContent = state
        app.appendChild(txt)
        bar.appendChild(app)
        document.body.appendChild(bar)
        const color = getComputedStyle(txt).color
        const bg = getComputedStyle(bar).backgroundColor
        results[state] = { color, bg, ratio: contrast(color, bg) }
        bar.remove()
      }
      return results
    },
    [...STATES]
  )
}

test.describe('#1649 titlebar connection-status text meets WCAG AA (4.5:1) in both themes', () => {
  test('every titlebar-app-state color (connected/connecting/disconnected) clears 4.5:1 against the titlebar background, light and dark', async () => {
    const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-1649-'))
    const udd = mkdtempSync(join(tmpdir(), 'bpmux-1649-udd-'))
    const { app } = await launchIsolatedElectron({ dataBase, userDataDir: udd })
    const win = await app.firstWindow()
    try {
      const failures: string[] = []
      for (const theme of ['light', 'dark'] as const) {
        const measured = await measureStateTextContrast(win, theme)
        for (const state of STATES) {
          const m = measured[state]
          if (m.ratio < 4.5) {
            failures.push(
              `${theme}/${state}: ${m.color} on ${m.bg} = ${m.ratio.toFixed(2)}:1 (needs 4.5)`
            )
          }
        }
      }
      expect(
        failures,
        `titlebar-app-state text below WCAG 1.4.3 AA 4.5:1: ${failures.join(' | ')}`
      ).toEqual([])
    } finally {
      await app.close()
    }
  })
})
