import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { issueJson, startMockGhServer } from './helpers/mock_gh'
import { GithubSettingsPage } from './pages/githubSettingsPage'

test.describe('enterprise connect — the form-typed creds reach GitHub, front to back', () => {
  test('checking Enterprise, typing creds, and connecting syncs the board and the mock sees the bearer', async () => {
    const mockGh = await startMockGhServer()
    mockGh.setIssues([issueJson(7171, 'Enterprise form proof story', [], null)])

    const { app, backendPort } = await launchIsolatedElectron({ unconnected: true })
    const win = await app.firstWindow()
    const settings = new GithubSettingsPage(win)

    await settings.openButton().click()
    await expect(settings.connectionState()).toHaveAttribute('data-state', 'disconnected')

    await settings.enterprise().check()
    await settings.baseUrl().fill(mockGh.baseUrl)
    await settings.owner().fill('acme')
    await settings.repo().fill('bpmux')
    await settings.token().fill('ghp_e2e_enterprise_secret')
    await settings.connect().click()

    await expect(settings.connectionState()).toHaveAttribute('data-state', 'connected')
    await expect(settings.connectionState()).toContainText('acme/bpmux')

    const { authToken } = await win.evaluate(() => ({
      authToken: (window as unknown as { bpmux?: { authToken?: string } }).bpmux?.authToken ?? ''
    }))
    await expect
      .poll(
        async () => {
          const board = await fetch(`http://127.0.0.1:${backendPort}/api/rpc/view_board`, {
            method: 'POST',
            headers: {
              'content-type': 'application/json',
              authorization: `Bearer ${authToken}`,
              'x-bpmux-agent': 'pm'
            },
            body: JSON.stringify({})
          })
          if (board.status !== 200) return ''
          return JSON.stringify(await board.json())
        },
        { timeout: 15_000 }
      )
      .toContain('Enterprise form proof story')

    const authorizations = mockGh
      .requestLog()
      .filter((r) => r.path.includes('/repos/acme/bpmux'))
      .map((r) => r.authorization ?? '')
    expect(
      authorizations.some((a) => a.includes('ghp_e2e_enterprise_secret')),
      `the mock must see the form-typed token on its requests; saw: ${JSON.stringify([...new Set(authorizations)])}`
    ).toBe(true)

    await app.close()
  })
})
