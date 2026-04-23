import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'

// #377: Ctrl+F inside an editable field must pass through to the
// browser / emacs native forward-char. The AppShell global keydown
// handler was intercepting Cmd/Ctrl+F unconditionally, overriding
// the chat textarea's expected navigation. Adding an `inEditable`
// guard (matching the Ctrl+P pattern in #331) restores native
// behavior for typing flow while preserving search-overlay
// keyboard access from non-editable focus.

async function launchWithFixture(): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-377-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-377-'))
  const { app } = await launchIsolatedElectron({ sessionBase, userDataDir: udd })
  const win = await app.firstWindow()
  await win.route('**/api/board', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ fetched_at: '2026-04-23T11:00:00Z', stories: [] })
    })
  })
  await win.route('**/api/messages*', async (route, request) => {
    const url = request.url()
    if (url.includes('message_type=activity')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: '[]'
      })
      return
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: '[]'
    })
  })
  await win.reload()
  return { app }
}

test.describe('desk-check #377 Ctrl+F focus-aware dispatch', () => {
  test('Ctrl+F inside chat input does NOT open search overlay', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Focus the chat input + type a marker so we can verify the
    // browser's native Ctrl+F didn't clobber focus or input state.
    const input = win.locator('[data-test="chat-input-textarea"]').first()
    await input.focus()
    await input.fill('hello world')

    await input.press('Control+f')

    // Search overlay must NOT be visible when focus was in input.
    const searchOverlay = win.locator('[data-test="search-overlay"]')
    await expect(searchOverlay).toHaveCount(0)

    // Input still holds our typed text (not cleared by the intercept).
    await expect(input).toHaveValue('hello world')

    await app.close()
  })

  test('Ctrl+F outside editable opens search overlay (existing behavior preserved)', async () => {
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    // Blur any autofocused input by clicking a non-editable region.
    // The app root wrapper is a good non-editable anchor.
    const root = win.locator('[data-test="app-root"]')
    await root.click({ position: { x: 5, y: 5 } })

    // Make sure focus isn't in an editable element anymore.
    const activeIsEditable = await win.evaluate(() => {
      const el = document.activeElement as HTMLElement | null
      return (
        el instanceof HTMLTextAreaElement ||
        el instanceof HTMLInputElement ||
        el?.isContentEditable === true
      )
    })
    expect(activeIsEditable).toBe(false)

    await win.keyboard.press('Control+f')

    // Search overlay opens.
    const searchOverlay = win.locator('[data-test="search-overlay"]')
    await expect(searchOverlay).toBeVisible({ timeout: 3_000 })

    await app.close()
  })

  test('Cmd+F in editable field OPENS search (Mac platform convention)', async () => {
    // Split-by-tradition (pm + designer direction 11:27 UTC):
    //   - Ctrl+F = emacs forward-char (guard in editable)
    //   - Cmd+F = Mac convention, opens search everywhere
    // Mac users reflexively Cmd+F to search even when typing; the
    // guard doesn't apply to Cmd. Only the Ctrl-tradition key gets
    // emacs-bindings treatment.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    await expect(win.locator('[data-test="chat-region"]')).toBeVisible()

    const input = win.locator('[data-test="chat-input-textarea"]').first()
    await input.focus()
    await input.fill('meta-test')

    await input.press('Meta+f')

    // Search overlay opens despite input having focus.
    const searchOverlay = win.locator('[data-test="search-overlay"]')
    await expect(searchOverlay).toBeVisible({ timeout: 3_000 })

    await app.close()
  })
})
