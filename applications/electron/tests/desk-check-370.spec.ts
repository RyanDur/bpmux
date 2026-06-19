import { test, expect, _electron as electron } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchIsolatedElectron } from './helpers/launch'
import { ChatPage } from './pages/chatPage'

// #370 rework: retire hardcoded role descriptions. Per Ryan's
// 2026-04-22 23:28 UTC comment: "agents should define themselves.
// this app needs to be usable to any type of agent and human
// collaboration." The static `ROLE_INFO[role].description` table
// named specific agents (pm/dev/designer/ryan); adding a new role
// would render as "unknown" with no meaningful text.
//
// Scope-narrow (per pm + designer endorsement on option-a): the
// tooltip now shows label (runtime-derived short-role, capitalized)
// + presence status. Rich self-descriptions become future work when
// an agent-driven `get_identity` metadata channel wires through
// to the renderer. The hardcoded constants file is deleted.

const AD_HOC_MESSAGES = [
  {
    from: 'ryan:electron',
    text: 'hello',
    ts: '2026-04-23T11:00:00Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  },
  {
    from: 'ops:claude-code',
    text: 'new role not in pre-#370 hardcoded list',
    ts: '2026-04-23T11:00:10Z',
    to: null,
    addressee: null,
    message_type: 'chat'
  }
]

const launchWithFixture = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const sessionBase = mkdtempSync(join(tmpdir(), 'bpmux-desk-check-370-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-370-'))
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
      body: JSON.stringify(AD_HOC_MESSAGES)
    })
  })
  await win.reload()
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
