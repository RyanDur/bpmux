import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from '../helpers/launch'

interface ContrastFailure {
  tag: string
  className: string
  testId: string | null
  snippet: string
  fg: string
  bg: string
  fontSizePx: number
  bold: boolean
  isLarge: boolean
  ratio: number
  requiredRatio: number
}

test.describe('#103 WCAG AA contrast — dark theme audit', () => {
  test('every visible text element meets WCAG AA contrast threshold (4.5:1 normal / 3:1 large)', async () => {
    const { app } = await launchIsolatedElectron()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    await win.evaluate(() => {
      window.localStorage.setItem('bpmux-theme', 'dark')
    })
    await win.reload()
    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
    await win.waitForFunction(() => document.documentElement.dataset.theme === 'dark')

    const failures = await win.evaluate(() => {
      function parseColor(cssColor: string): [number, number, number, number] | null {
        const m = /^rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)$/.exec(cssColor.trim())
        if (!m) return null
        return [Number(m[1]), Number(m[2]), Number(m[3]), m[4] !== undefined ? Number(m[4]) : 1]
      }

      function relativeLuminance(r: number, g: number, b: number): number {
        const channel = (c: number): number => {
          const s = c / 255
          return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
      }

      function contrastRatio(fg: [number, number, number], bg: [number, number, number]): number {
        const l1 = relativeLuminance(fg[0], fg[1], fg[2])
        const l2 = relativeLuminance(bg[0], bg[1], bg[2])
        const lighter = Math.max(l1, l2)
        const darker = Math.min(l1, l2)
        return (lighter + 0.05) / (darker + 0.05)
      }

      function effectiveBackground(el: Element): [number, number, number] {
        let cur: Element | null = el
        while (cur) {
          const parsed = parseColor(window.getComputedStyle(cur).backgroundColor)
          if (parsed && parsed[3] > 0) {
            return [parsed[0], parsed[1], parsed[2]]
          }
          cur = cur.parentElement
        }
        return [0, 0, 0]
      }

      function isElementVisible(el: Element): boolean {
        const cs = window.getComputedStyle(el)
        if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false
        const rect = (el as HTMLElement).getBoundingClientRect()
        return rect.width > 0 && rect.height > 0
      }

      const results: ContrastFailure[] = []
      const seen = new Set<Element>()
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
        acceptNode: (n) =>
          n.textContent?.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT
      })

      while (walker.nextNode()) {
        const el = walker.currentNode.parentElement
        if (!el || seen.has(el)) continue
        seen.add(el)
        if (!isElementVisible(el)) continue
        const cs = window.getComputedStyle(el)
        const fgParsed = parseColor(cs.color)
        if (!fgParsed) continue
        const fg: [number, number, number] = [fgParsed[0], fgParsed[1], fgParsed[2]]
        const bg = effectiveBackground(el)
        const ratio = contrastRatio(fg, bg)
        const fontSizePx = parseFloat(cs.fontSize)
        const weight = parseInt(cs.fontWeight, 10) || 400
        const bold = weight >= 700
        const isLarge = fontSizePx >= 24 || (bold && fontSizePx >= 18.66)
        const requiredRatio = isLarge ? 3 : 4.5
        if (ratio < requiredRatio) {
          results.push({
            tag: el.tagName.toLowerCase(),
            className: el.className.toString().slice(0, 80),
            testId: el.getAttribute('data-test'),
            snippet: (el.textContent ?? '').trim().slice(0, 60),
            fg: `rgb(${fg[0]}, ${fg[1]}, ${fg[2]})`,
            bg: `rgb(${bg[0]}, ${bg[1]}, ${bg[2]})`,
            fontSizePx,
            bold,
            isLarge,
            ratio: Math.round(ratio * 100) / 100,
            requiredRatio
          })
        }
      }
      return results
    })

    if (failures.length > 0) {
      console.log(`\n#103 contrast failures (${failures.length} elements):`)
      for (const f of failures) {
        console.log(
          `  <${f.tag}${f.testId ? ` data-test="${f.testId}"` : ''}${f.className ? ` class="${f.className}"` : ''}> ratio=${f.ratio} required=${f.requiredRatio} fg=${f.fg} bg=${f.bg} font=${f.fontSizePx}px${f.bold ? ' bold' : ''} text="${f.snippet}"`
        )
      }
    }

    expect(
      failures,
      `#103: ${failures.length} text element(s) fail WCAG AA contrast`
    ).toEqual([])

    await app.close()
  })
})
