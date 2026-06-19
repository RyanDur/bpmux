import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from '../helpers/launch'
import { formatViolations, runA11yAudit } from '../helpers/a11yAudit'
import { ChatPage } from '../pages/chatPage'

interface ContrastFailure {
  category: 'contrast'
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

interface AriaFailure {
  category: 'aria'
  tag: string
  className: string
  testId: string | null
  reason: string
}

interface KeyboardFailure {
  category: 'keyboard'
  tag: string
  className: string
  testId: string | null
  reason: string
}

interface FocusFailure {
  category: 'focus-visible'
  tag: string
  className: string
  testId: string | null
  reason: string
}

type AuditFailure = ContrastFailure | AriaFailure | KeyboardFailure | FocusFailure

test.describe('#425 a11y audit — dark-theme contrast + ARIA + keyboard + focus-visible', () => {
  test('every visible interactive + text element passes the four-axis audit', async () => {
    const { app } = await launchIsolatedElectron({
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'audit-fixture pm message',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'dev:claude-code',
          text: 'audit-fixture dev message',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'chat'
        }
      ],
      seedRoster: ['pm:claude-code', 'dev:claude-code']
    })
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(win.getByRole('main')).toBeVisible()
    await expect(chat.region()).toBeVisible()

    await win.evaluate(() => {
      window.localStorage.setItem('bpmux-theme', 'dark')
    })
    await win.reload()
    await expect(win.getByRole('main')).toBeVisible()
    await expect(chat.region()).toBeVisible()
    await win.waitForFunction(() => document.documentElement.dataset.theme === 'dark')
    await win.evaluate(() => {
      document.getAnimations().forEach((animation) => {
        if (animation.effect?.getComputedTiming().iterations !== Infinity) {
          animation.finish()
        }
      })
    })

    const failures = await win.evaluate(() => {
      const parseColor = (cssColor: string): [number, number, number, number] | null => {
        const m = /^rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)$/.exec(cssColor.trim())
        if (!m) return null
        return [Number(m[1]), Number(m[2]), Number(m[3]), m[4] !== undefined ? Number(m[4]) : 1]
      }
      const relativeLuminance = (r: number, g: number, b: number): number => {
        const channel = (c: number): number => {
          const s = c / 255
          return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
      }
      const contrastRatio = (
        fg: [number, number, number],
        bg: [number, number, number]
      ): number => {
        const l1 = relativeLuminance(fg[0], fg[1], fg[2])
        const l2 = relativeLuminance(bg[0], bg[1], bg[2])
        return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05)
      }
      const effectiveBackground = (el: Element): [number, number, number] => {
        let r = 0
        let g = 0
        let b = 0
        let remainingAlpha = 1
        let cur: Element | null = el
        while (cur && remainingAlpha > 0.001) {
          const parsed = parseColor(window.getComputedStyle(cur).backgroundColor)
          if (parsed && parsed[3] > 0) {
            const layerAlpha = parsed[3] * remainingAlpha
            r += parsed[0] * layerAlpha
            g += parsed[1] * layerAlpha
            b += parsed[2] * layerAlpha
            remainingAlpha *= 1 - parsed[3]
          }
          cur = cur.parentElement
        }
        if (remainingAlpha > 0.001) {
          r += 0 * remainingAlpha
          g += 0 * remainingAlpha
          b += 0 * remainingAlpha
        }
        return [Math.round(r), Math.round(g), Math.round(b)]
      }
      const isElementVisible = (el: Element): boolean => {
        const cs = window.getComputedStyle(el)
        if (cs.display === 'none' || cs.visibility === 'hidden' || cs.opacity === '0') return false
        const rect = (el as HTMLElement).getBoundingClientRect()
        return rect.width > 0 && rect.height > 0
      }
      const descriptor = (
        el: Element
      ): { tag: string; className: string; testId: string | null } => ({
        tag: el.tagName.toLowerCase(),
        className: el.className.toString().slice(0, 80),
        testId: el.getAttribute('data-test')
      })
      const accessibleName = (el: Element): string => {
        const labelledby = el.getAttribute('aria-labelledby')
        if (labelledby) {
          const ref = document.getElementById(labelledby)
          if (ref?.textContent?.trim()) return ref.textContent.trim()
        }
        const ariaLabel = el.getAttribute('aria-label')
        if (ariaLabel?.trim()) return ariaLabel.trim()
        const title = el.getAttribute('title')
        if (title?.trim()) return title.trim()
        const text = (el.textContent ?? '').trim()
        if (text) return text
        if (el.tagName === 'INPUT') {
          const id = el.getAttribute('id')
          if (id) {
            const explicitLabel = document.querySelector(`label[for="${id}"]`)
            if (explicitLabel?.textContent?.trim()) return explicitLabel.textContent.trim()
          }
          const wrappingLabel = el.closest('label')
          if (wrappingLabel?.textContent?.trim()) return wrappingLabel.textContent.trim()
          const placeholder = el.getAttribute('placeholder')
          if (placeholder?.trim()) return placeholder.trim()
        }
        return ''
      }
      const out: Array<{
        category: string
        tag: string
        className: string
        testId: string | null
        [extra: string]: unknown
      }> = []

      const seenForContrast = new Set<Element>()
      const textWalker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
        acceptNode: (n) =>
          n.textContent?.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT
      })
      while (textWalker.nextNode()) {
        const el = textWalker.currentNode.parentElement
        if (!el || seenForContrast.has(el)) continue
        seenForContrast.add(el)
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
          out.push({
            category: 'contrast',
            ...descriptor(el),
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

      const interactiveSelector =
        'button, a[href], input, select, textarea, [role="button"], [role="link"], [role="tab"], [role="menuitem"], [role="checkbox"], [role="radio"], [role="switch"], [tabindex]'
      const interactives = Array.from(document.querySelectorAll(interactiveSelector)).filter((el) =>
        isElementVisible(el)
      )

      for (const el of interactives) {
        if (el.getAttribute('aria-hidden') === 'true') continue
        if (!accessibleName(el)) {
          out.push({
            category: 'aria',
            ...descriptor(el),
            reason: 'no accessible name (aria-label / aria-labelledby / text / title / label[for])'
          })
        }
      }

      for (const el of interactives) {
        const tabindex = el.getAttribute('tabindex')
        if (tabindex !== null && Number.parseInt(tabindex, 10) < 0) {
          if (el.getAttribute('aria-hidden') === 'true') continue
          out.push({
            category: 'keyboard',
            ...descriptor(el),
            reason: `tabindex=${tabindex} excludes from tab-order; either remove tabindex or set aria-hidden`
          })
        }
      }

      const focusSnapshot = (
        el: Element
      ): {
        outline: string
        boxShadow: string
        borderColor: string
        backgroundColor: string
      } => {
        const cs = window.getComputedStyle(el)
        return {
          outline: `${cs.outlineStyle} ${cs.outlineWidth} ${cs.outlineColor}`,
          boxShadow: cs.boxShadow,
          borderColor: `${cs.borderTopColor} ${cs.borderRightColor} ${cs.borderBottomColor} ${cs.borderLeftColor}`,
          backgroundColor: cs.backgroundColor
        }
      }
      const declaresFocusVisibleStyle = (el: Element): boolean => {
        const walk = (rules: CSSRuleList): boolean => {
          for (const rule of Array.from(rules)) {
            if (rule instanceof CSSStyleRule) {
              const sel = rule.selectorText
              if (sel.includes(':focus')) {
                for (const fragment of sel.split(',')) {
                  const stripped = fragment.replace(/:focus(-visible)?/g, '').trim()
                  if (!stripped) continue
                  try {
                    if (el.matches(stripped)) return true
                  } catch {
                    continue
                  }
                }
              }
            }
            const nested = (rule as unknown as { cssRules?: CSSRuleList }).cssRules
            if (nested && walk(nested)) return true
          }
          return false
        }
        for (const sheet of Array.from(document.styleSheets)) {
          let rules: CSSRuleList
          try {
            rules = sheet.cssRules
          } catch {
            continue
          }
          if (walk(rules)) return true
        }
        return false
      }
      for (const el of interactives) {
        if ((el as HTMLElement).hasAttribute('disabled')) continue
        if (el.getAttribute('aria-hidden') === 'true') continue
        const previousActive = document.activeElement as HTMLElement | null
        ;(el as HTMLElement).blur()
        const restingSnap = focusSnapshot(el)
        try {
          ;(el as HTMLElement).focus()
        } catch {
          continue
        }
        if (document.activeElement !== el) {
          previousActive?.focus()
          continue
        }
        const focusSnap = focusSnapshot(el)
        const cs = window.getComputedStyle(el)
        const outlineWidth = parseFloat(cs.outlineWidth)
        const hasOutline = cs.outlineStyle !== 'none' && outlineWidth > 0
        const hasBoxShadow = cs.boxShadow !== 'none' && cs.boxShadow.trim().length > 0
        const sameAsResting =
          restingSnap.outline === focusSnap.outline &&
          restingSnap.boxShadow === focusSnap.boxShadow &&
          restingSnap.borderColor === focusSnap.borderColor &&
          restingSnap.backgroundColor === focusSnap.backgroundColor
        const hasFocusVisibleRule = declaresFocusVisibleStyle(el)
        if (!hasOutline && !hasBoxShadow && sameAsResting && !hasFocusVisibleRule) {
          out.push({
            category: 'focus-visible',
            ...descriptor(el),
            reason: `no focus indicator: outline/box-shadow/border/background unchanged on focus + no :focus or :focus-visible rule targets element`
          })
        }
        previousActive?.focus()
      }

      return out
    })

    if (failures.length > 0) {
      console.log(`\n#425 a11y audit failures (${failures.length}):`)
      for (const f of failures as unknown as AuditFailure[]) {
        const id = f.testId ? ` data-test="${f.testId}"` : ''
        const cls = f.className ? ` class="${f.className}"` : ''
        if (f.category === 'contrast') {
          console.log(
            `  [contrast] <${f.tag}${id}${cls}> ratio=${f.ratio} required=${f.requiredRatio} fg=${f.fg} bg=${f.bg} font=${f.fontSizePx}px${f.bold ? ' bold' : ''} text="${f.snippet}"`
          )
        } else {
          console.log(`  [${f.category}] <${f.tag}${id}${cls}> ${f.reason}`)
        }
      }
    }

    expect(failures, `#425: ${failures.length} a11y violation(s) — see test output`).toEqual([])

    const axeResult = await runA11yAudit(win)
    if (axeResult.violations.length > 0) {
      console.log(`\n#425 axe-core a11y audit:\n${formatViolations(axeResult)}`)
    }
    expect(
      axeResult.violations.map((v) => ({ id: v.id, impact: v.impact })),
      `#425 axe-core: ${axeResult.violations.length} violation(s) — see test output for full detail`
    ).toEqual([])

    await app.close()
  })
})
