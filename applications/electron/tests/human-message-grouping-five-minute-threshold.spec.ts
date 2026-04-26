import { test, expect } from '@playwright/test'
import { launchIsolatedWithWindow } from './helpers/launch'

test.describe('human-side E2E #476 — read-class — message grouping 5-min threshold', () => {
  test('two same-author chat messages within 5min collapse the second into a grouped row', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'GROUPED_FIRST_MARKER_BPF18',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'pm:claude-code',
          text: 'GROUPED_SECOND_MARKER_BPF18',
          ts: '2026-04-22T16:01:00Z',
          message_type: 'chat'
        }
      ]
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toBeVisible()
    await expect(list).toContainText('GROUPED_FIRST_MARKER_BPF18', { timeout: 10000 })
    await expect(list).toContainText('GROUPED_SECOND_MARKER_BPF18')

    consoleErrors.length = 0
    pageErrors.length = 0

    const firstRows = list.locator('.message-row--first')
    const groupedRows = list.locator('.message-row--grouped')
    await expect(firstRows).toHaveCount(1)
    await expect(groupedRows).toHaveCount(1)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })

  test('two same-author chat messages 6min apart render as two first-rows, not grouped', async () => {
    const { app, win } = await launchIsolatedWithWindow({
      extraEnv: { BPMUX_TEST_MODE: '1' },
      seedMessages: [
        {
          from: 'pm:claude-code',
          text: 'NONGROUPED_FIRST_MARKER_BPF18',
          ts: '2026-04-22T16:00:00Z',
          message_type: 'chat'
        },
        {
          from: 'pm:claude-code',
          text: 'NONGROUPED_SECOND_MARKER_BPF18',
          ts: '2026-04-22T16:06:00Z',
          message_type: 'chat'
        }
      ]
    })

    const consoleErrors: string[] = []
    const pageErrors: Error[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    win.on('pageerror', (e) => pageErrors.push(e))

    const list = win.locator('[data-test="message-list"]')
    await expect(list).toBeVisible()
    await expect(list).toContainText('NONGROUPED_SECOND_MARKER_BPF18', { timeout: 10000 })

    consoleErrors.length = 0
    pageErrors.length = 0

    const firstRows = list.locator('.message-row--first')
    const groupedRows = list.locator('.message-row--grouped')
    await expect(firstRows).toHaveCount(2)
    await expect(groupedRows).toHaveCount(0)

    expect(consoleErrors).toEqual([])
    expect(pageErrors).toEqual([])

    await app.close()
  })
})
