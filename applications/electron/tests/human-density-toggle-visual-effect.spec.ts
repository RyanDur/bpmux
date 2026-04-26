import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

// Read the resolved CSS custom-property the chat avatar branches off
// of. This is the user-observable visual change — when density flips
// from "comfortable" → "compact" the avatar shrinks from 28px → 20px
// (per styles/tokens.css). Asserting this catches CSS regressions
// that an attribute-only test would miss.
const readAvatarSizeVar = (win: import('@playwright/test').Page): Promise<string> =>
  win.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue('--chat-avatar-size').trim()
  )

test.describe('human-side E2E #476 — read-class — density toggle visual', () => {
  test('density toggle flips a user-observable CSS variable (chat avatar size)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="titlebar"]')).toBeVisible()

    const initialAttr = await win.evaluate(() => document.documentElement.dataset.density)
    expect(initialAttr === 'compact' || initialAttr === 'comfortable').toBe(true)
    const initialSize = await readAvatarSizeVar(win)
    expect(initialSize).toMatch(/^\d+px$/)

    consoleErrors.length = 0
    pageErrors.length = 0

    const button = win.locator('[data-test="titlebar-density-toggle"]')
    await button.click()
    const toggled = initialAttr === 'compact' ? 'comfortable' : 'compact'
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe(toggled)
    await expect.poll(() => readAvatarSizeVar(win)).not.toBe(initialSize)

    await button.click()
    await expect
      .poll(() => win.evaluate(() => document.documentElement.dataset.density))
      .toBe(initialAttr)
    await expect.poll(() => readAvatarSizeVar(win)).toBe(initialSize)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('compact density resolves --chat-avatar-size to 20px (per tokens.css)', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' }
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await expect(win.locator('[data-test="titlebar"]')).toBeVisible()
    const button = win.locator('[data-test="titlebar-density-toggle"]')
    if ((await button.getAttribute('data-density')) !== 'compact') {
      await button.click()
    }

    consoleErrors.length = 0
    pageErrors.length = 0

    await expect(button).toHaveAttribute('data-density', 'compact')
    await expect.poll(() => readAvatarSizeVar(win)).toBe('20px')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
