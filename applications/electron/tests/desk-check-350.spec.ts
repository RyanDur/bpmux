import { test, expect, _electron as electron, type Page } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const launchIsolated = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-350-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-350-'))
  const { app } = await launchIsolatedElectron({ dataBase, userDataDir: udd })
  return { app }
}

const readToken = async (win: Page, name: string): Promise<string> =>
  win.evaluate(
    (n: string) => window.getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    name
  )

test.describe('desk-check #350 motion substrate', () => {
  test('six motion tokens resolve to declared values in default media', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const tokens = [
      '--motion-duration-instant',
      '--motion-duration-fast',
      '--motion-duration-medium',
      '--motion-ease-out',
      '--motion-ease-in',
      '--motion-ease-in-out'
    ]

    const values: Record<string, string> = {}
    for (const t of tokens) {
      values[t] = await readToken(win, t)
    }

    console.log('#350 AC1 motion tokens (default media):')
    for (const [k, v] of Object.entries(values)) {
      console.log(`  ${k}: "${v}"`)
    }

    // Expected values per spec.
    expect(values['--motion-duration-instant']).toBe('80ms')
    expect(values['--motion-duration-fast']).toBe('150ms')
    expect(values['--motion-duration-medium']).toBe('250ms')
    // Easings are cubic-beziers — just assert they're non-empty + start with cubic-bezier(.
    expect(values['--motion-ease-out']).toContain('cubic-bezier')
    expect(values['--motion-ease-in']).toContain('cubic-bezier')
    expect(values['--motion-ease-in-out']).toContain('cubic-bezier')

    await app.close()
  })

  test('duration tokens zero under prefers-reduced-motion:reduce', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    await win.emulateMedia({ reducedMotion: 'reduce' })

    const instant = await readToken(win, '--motion-duration-instant')
    const fast = await readToken(win, '--motion-duration-fast')
    const medium = await readToken(win, '--motion-duration-medium')

    console.log('#350 AC2 duration tokens under reduced-motion:reduce:')
    console.log(`  --motion-duration-instant: "${instant}" (expect 0ms)`)
    console.log(`  --motion-duration-fast: "${fast}" (expect 0ms)`)
    console.log(`  --motion-duration-medium: "${medium}" (expect 0ms)`)

    expect(instant).toBe('0ms')
    expect(fast).toBe('0ms')
    expect(medium).toBe('0ms')

    await app.close()
  })

  test('motion-safe-transform utility: transform reset + transition-property opacity under reduced-motion', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    // Inject a test element with motion-safe-transform + a transform.
    // Evaluate its computed style in both media states.
    const normal = await win.evaluate(() => {
      const el = document.createElement('div')
      el.className = 'motion-safe-transform'
      el.style.transform = 'translateY(8px)'
      el.style.transitionProperty = 'transform, opacity'
      document.body.appendChild(el)
      const cs = window.getComputedStyle(el)
      const r = { transform: cs.transform, transitionProperty: cs.transitionProperty }
      el.remove()
      return r
    })
    console.log(
      `#350 AC3 normal media: transform=${normal.transform}, transition-property=${normal.transitionProperty}`
    )

    await win.emulateMedia({ reducedMotion: 'reduce' })

    const reduced = await win.evaluate(() => {
      const el = document.createElement('div')
      el.className = 'motion-safe-transform'
      el.style.transform = 'translateY(8px)'
      el.style.transitionProperty = 'transform, opacity'
      document.body.appendChild(el)
      const cs = window.getComputedStyle(el)
      const r = { transform: cs.transform, transitionProperty: cs.transitionProperty }
      el.remove()
      return r
    })
    console.log(
      `#350 AC3 reduced motion: transform=${reduced.transform}, transition-property=${reduced.transitionProperty}`
    )

    // Under reduced-motion, .motion-safe-transform should force
    // transform:none and transition-property:opacity.
    expect(reduced.transform).toBe('none')
    expect(reduced.transitionProperty).toBe('opacity')

    await app.close()
  })

  test('no text-floor regression (universal-property still holds)', async () => {
    // The #348-rework text-walker check is already part of the
    // real-backend suite; this test confirms #350's token-only
    // substrate change didn't regress it. We sample a few critical
    // surfaces visually.
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).region()).toBeVisible()
    await win.waitForTimeout(300)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '50-app-unchanged-with-motion-tokens.png')
    })
    // Spot check a few computed font sizes to confirm the a11y
    // floor still holds with #350 applied.
    const sizes = await win.evaluate(() => {
      const pick = ['.sidebar-item', '.filter-pill', '.message-from', '.board-title']
      return pick.map((sel) => {
        const el = document.querySelector(sel)
        return el
          ? { sel, fontSize: parseFloat(window.getComputedStyle(el).fontSize) }
          : { sel, fontSize: null }
      })
    })
    console.log(`#350 AC5 font-size spot check:`)
    for (const s of sizes) {
      console.log(`  ${s.sel}: ${s.fontSize}px`)
      if (s.fontSize !== null) {
        expect(s.fontSize, `${s.sel} should be ≥12px`).toBeGreaterThanOrEqual(12)
      }
    }
    await app.close()
  })
})
