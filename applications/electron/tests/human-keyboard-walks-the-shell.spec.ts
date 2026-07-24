import { expect, test } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

test('tab always moves focus forward — no control wedges the keyboard', async () => {
  test.setTimeout(120_000)
  const { app } = await launchIsolatedElectron()
  const win = await app.firstWindow()
  await expect(new ChatPage(win).appRoot()).toBeVisible()

  const visited: string[] = []
  for (let press = 0; press < 40; press++) {
    await win.keyboard.press('Tab')
    const step = await win.evaluate(() => {
      const previous = document.querySelector('[data-kbd-cursor]')
      const current = document.activeElement
      const moved = current !== null && current !== previous
      previous?.removeAttribute('data-kbd-cursor')
      current?.setAttribute('data-kbd-cursor', '')
      const label =
        current?.getAttribute('aria-label') ??
        current?.getAttribute('data-test') ??
        `${current?.tagName}.${current?.className}`
      return { moved, label }
    })
    expect(step.moved, `focus wedged on "${step.label}" after ${press + 1} tabs`).toBe(true)
    visited.push(step.label)
  }

  const walked = visited.join('\n')
  expect(walked).toMatch(/channel|sidebar/i)
  expect(walked).toMatch(/chat|message/i)

  await app.close()
})
