import { type Locator, type Page, expect } from '@playwright/test'

export const runCommandUntilOutput = async (
  win: Page,
  command: string,
  readback: Locator,
  expectedOutput: string,
  timeout = 20000
): Promise<void> => {
  await expect
    .poll(
      async () => {
        await win.keyboard.type(command)
        await win.keyboard.press('Enter')
        return (await readback.textContent()) ?? ''
      },
      { timeout, intervals: [400, 700, 1000, 1500] }
    )
    .toContain(expectedOutput)
}
