import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from '../helpers/launch'

test.describe('#1242 chat-paste — zero console errors when unfurl renders', () => {
  test('mocked unfurl with image+favicon URLs does NOT trigger external image-fetch in renderer', async () => {
    const seedMessages = [
      {
        from: 'pm:claude-code',
        text: 'check out https://code.claude.com/docs/en/sub-agents for the spec',
        ts: '2026-05-09T17:20:00Z',
        message_type: 'chat'
      }
    ]
    const { app } = await launchIsolatedElectron({ seedMessages })
    const win = await app.firstWindow()

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    await win.route('**/api/unfurl**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          url: 'https://code.claude.com/docs/en/sub-agents',
          title: 'Create custom subagents',
          description: 'Create and use specialized AI subagents.',
          site_name: 'Claude Code',
          image: 'https://claude-code.mintlify.app/_next/image?url=external-og.png',
          favicon: 'https://code.claude.com/docs/_mintlify/favicons/favicon-16x16.png'
        })
      })
    })

    await expect(win.locator('[data-test="app-root"]')).toBeVisible()
    const card = win.locator('[data-test="unfurl-card-https://code.claude.com/docs/en/sub-agents"]')
    await expect(card).toBeVisible({ timeout: 10_000 })

    consoleErrors.length = 0
    pageErrors.length = 0
    await win.waitForTimeout(500)

    const externalImgCount = await win.locator('.unfurl-card img').count()
    expect(externalImgCount).toBe(0)

    await expect(card).toContainText('Create custom subagents')
    await expect(card).toContainText('Claude Code')

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
