import { test, expect, _electron as electron } from '@playwright/test'

test('launches + connects + renders chat', async () => {
  const app = await electron.launch({ args: ['out/main/index.js'] })
  const win = await app.firstWindow()
  await expect(win.locator('[data-test="app-root"]')).toBeVisible()
  // Once connected, the chat region should be present
  await expect(win.locator('[data-test="chat-region"]')).toBeVisible()
  await app.close()
})
