import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron, waitForMessagesLoaded } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

const AD_HOC_MESSAGES = [
  {
    from: 'owner:electron',
    text: 'hello',
    ts: '2026-04-23T11:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'ops:claude-code',
    text: 'new role not in pre-#370 hardcoded list',
    ts: '2026-04-23T11:00:10Z',
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-370-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-370-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    seedMessages: AD_HOC_MESSAGES
  })
  const win = await app.firstWindow()
  await waitForMessagesLoaded(win, 'new role not in pre-#370 hardcoded list')
  return { app }
}

test.describe('desk-check #370 avatar tooltip (rework — no hardcoded descriptions)', () => {
  test('tooltip carries label + status but NO description element', async () => {
    // Falsifier (a): tooltip still pulls text from ROLE_INFO /
    // roleInfoFor. The description element used to render a static
    // "pm — product backlog + sequencing…" line. Absence of that
    // element is the regression gate.
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)
    await expect(chat.region()).toBeVisible()

    const tooltip = chat.tooltips().first()
    await expect(tooltip).toBeAttached()

    // Label present (role name, capitalized).
    await expect(tooltip.locator('.avatar-tooltip-label')).toBeVisible()

    // Status present.
    await expect(tooltip.locator('.avatar-tooltip-status')).toBeVisible()

    // Description element retired — no .avatar-tooltip-desc anywhere.
    await expect(win.locator('.avatar-tooltip-desc')).toHaveCount(0)
  })

  test('new role (ops) renders tooltip label from runtime sender, not constants', async () => {
    // Falsifier (b): adding a new role renders "unknown" fallthrough.
    // Pre-#370 rework, `ops` would have hit UNKNOWN_ROLE_INFO and
    // displayed label="unknown." Post-rework, label comes straight
    // from the sender short-name, capitalized — so ops shows "Ops."
    const { app } = await launchWithFixture()
    const win = await app.firstWindow()
    const chat = new ChatPage(win)

    const opsMessageRow = chat.messageRow('new role not in')
    await expect(opsMessageRow).toBeVisible()
    const opsAvatar = opsMessageRow.getByRole('button', { name: /^Ops — / })
    await expect(opsAvatar).toBeVisible()

    const opsTooltipLabel = opsMessageRow.locator('.avatar-tooltip-label').first()
    await expect(opsTooltipLabel).toHaveText(/^Ops$/)

    const ariaLabel = await opsAvatar.getAttribute('aria-label')
    expect(ariaLabel).toMatch(/^Ops — /)

    await app.close()
  })
})
