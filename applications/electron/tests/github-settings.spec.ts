import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { GithubSettingsPage } from './pages/githubSettingsPage'

test.describe('GitHub settings — the owner connects, reconfigures, and disconnects from the UI', () => {
  test('the seeded connection shows, a reconfigure replaces it, and disconnect leaves it unconnected', async () => {
    const { app } = await launchIsolatedElectron()
    const win = await app.firstWindow()
    const settings = new GithubSettingsPage(win)

    await settings.openButton().click()
    await expect(settings.dialog()).toBeVisible()
    await expect(settings.connectionState()).toHaveAttribute('data-state', 'connected')
    await expect(settings.connectionState()).toContainText('acme/bpmux')

    await settings.owner().fill('RyanDur')
    await settings.repo().fill('sand')
    await settings.token().fill('ghp_e2e_fixture')
    await settings.connect().click()
    await expect(settings.connectionState()).toContainText('RyanDur/sand')

    await settings.disconnect().click()
    await expect(settings.connectionState()).toHaveAttribute('data-state', 'disconnected')
    await expect(settings.connectionState()).toContainText('not connected')

    await app.close()
  })

  test('validation errors paint into reserved slots — the dialog never changes height', async () => {
    const { app } = await launchIsolatedElectron({ unconnected: true })
    const win = await app.firstWindow()
    const settings = new GithubSettingsPage(win)

    await settings.openButton().click()
    await settings.enterprise().check()
    const before = await settings.dialog().boundingBox()

    await settings.connect().click()
    await expect(win.locator('#gh-owner-error')).toHaveText('enter the owner')
    const after = await settings.dialog().boundingBox()
    expect(after?.height).toBe(before?.height)

    await app.close()
  })
})
