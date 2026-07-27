import { test, expect } from '@playwright/test'
import { launchIsolatedElectron } from './helpers/launch'
import { issueJson, startMockGhServer } from './helpers/mock_gh'

test.describe('live connect — syncing starts when the creds land, no restart', () => {
  test('an unconnected server starts syncing the moment the connection is submitted', async () => {
    const mockGh = await startMockGhServer()
    mockGh.setIssues([issueJson(4242, 'Live-connect proof story', [], null)])

    const { app, backendPort } = await launchIsolatedElectron({ unconnected: true })
    const win = await app.firstWindow()
    const apiBase = `http://127.0.0.1:${backendPort}`
    const { authToken, ownerNonce } = await win.evaluate(() => {
      const bridge = (window as unknown as { bpmux?: { authToken?: string; ownerNonce?: string } })
        .bpmux
      return { authToken: bridge?.authToken ?? '', ownerNonce: bridge?.ownerNonce ?? '' }
    })
    const headers = {
      'content-type': 'application/json',
      authorization: `Bearer ${authToken}`,
      'x-bpmux-owner-nonce': ownerNonce
    }

    const before = await fetch(`${apiBase}/api/github-connection`, { headers })
    expect(((await before.json()) as { state: string }).state).toBe('disconnected')

    const connect = await fetch(`${apiBase}/api/github-connection`, {
      method: 'PUT',
      headers,
      body: JSON.stringify({ owner: 'acme', repo: 'bpmux', base_url: mockGh.baseUrl })
    })
    expect(connect.status).toBe(200)

    await expect
      .poll(
        async () => {
          const board = await fetch(`${apiBase}/api/rpc/view_board`, {
            method: 'POST',
            headers: { ...headers, 'x-bpmux-agent': 'pm' },
            body: JSON.stringify({})
          })
          if (board.status !== 200) return ''
          return JSON.stringify(await board.json())
        },
        { timeout: 15_000 }
      )
      .toContain('Live-connect proof story')

    await app.close()
  })
})
