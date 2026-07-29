import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { startMockGhServer } from './helpers/mock_gh'
import { GithubSettingsPage } from './pages/githubSettingsPage'

test.describe('a token GitHub refuses surfaces in the dialog, front to back', () => {
  test('connect with refused creds → the dialog reports the failing sync', async () => {
    const mockGh = await startMockGhServer()
    mockGh.on({
      method: 'GET',
      match: /^\/repos\/acme\/bpmux\/issues/,
      status: 401,
      bodyJson: { message: 'Bad credentials' }
    })

    const { app } = await launchIsolatedElectron({ unconnected: true })
    const win = await app.firstWindow()
    const settings = new GithubSettingsPage(win)

    await settings.openButton().click()
    await settings.enterprise().check()
    await settings.baseUrl().fill(mockGh.baseUrl)
    await settings.owner().fill('acme')
    await settings.repo().fill('bpmux')
    await settings.token().fill('ghp_refused_secret')
    await settings.connect().click()

    await expect(settings.connectionState()).toHaveAttribute('data-state', 'connected')
    await expect(win.locator('[data-test="gh-sync-error"]')).toBeVisible({ timeout: 20_000 })
    await expect(win.locator('[data-test="gh-sync-error"]')).toContainText('sync is failing')

    await app.close()
    await mockGh.close()
  })
})
