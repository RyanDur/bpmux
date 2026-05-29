import { expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchWithMockGh } from './launch'
import type { LaunchOpts, LaunchResult, MockGhLaunchResult } from './launch'
import type { Page } from '@playwright/test'

export const MOCK_CURRENT_ITERATION_NUMBER = 999

export const mockCurrentIteration = (
  themeName: string,
  createdAt: string
): Record<string, unknown> => ({
  number: MOCK_CURRENT_ITERATION_NUMBER,
  title: `Iteration ${themeName}`,
  state: 'in-progress',
  labels: ['iteration'],
  assignees: [],
  created_at: createdAt,
  body: '',
  is_current: true
})

export const inMockCurrentIteration = (position: number): Record<string, unknown> => ({
  iteration_membership: {
    iteration_number: MOCK_CURRENT_ITERATION_NUMBER,
    position
  }
})

export type RpcFn = (tool: string, body: unknown, agent?: string) => Promise<Response>

export const seedCurrentIterationViaRpc = async (
  rpc: RpcFn,
  themeName: string
): Promise<number> => {
  const iterResp = await rpc('create_iteration', { theme_name: themeName }, 'pm')
  expect(iterResp.status).toBe(200)
  const iterJson = (await iterResp.json()) as { message?: string }
  const iterNumber = Number(iterJson.message?.match(/#(\d+)/)?.[1])
  expect(iterNumber).toBeGreaterThan(0)

  const designateResp = await rpc(
    'designate_current_iteration',
    { iteration_number: iterNumber },
    'pm'
  )
  expect(designateResp.status).toBe(200)

  return iterNumber
}

export const addStoryToIterationViaRpc = async (
  rpc: RpcFn,
  storyNumber: number,
  iterationNumber: number,
  position: number
): Promise<void> => {
  const addResp = await rpc(
    'add_story_to_iteration',
    { story_number: storyNumber, iteration_number: iterationNumber, position },
    'pm'
  )
  expect(addResp.status).toBe(200)
}

export const setupCurrentIterationViaRpc = async (
  rpc: RpcFn,
  storyNumber: number,
  themeName: string
): Promise<number> => {
  const iterNumber = await seedCurrentIterationViaRpc(rpc, themeName)
  await addStoryToIterationViaRpc(rpc, storyNumber, iterNumber, 1)
  return iterNumber
}

export const seedCurrentIterationViaTestApi = async (
  backendPort: number,
  authToken: string,
  themeName: string
): Promise<number> => {
  const post = async (path: string, body: unknown): Promise<{ message?: string }> => {
    const res = await fetch(`http://127.0.0.1:${backendPort}/api/test/${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${authToken}` },
      body: JSON.stringify(body)
    })
    expect(res.ok).toBe(true)
    return res.json() as Promise<{ message?: string }>
  }
  const create = await post('create_iteration', { theme_name: themeName, agent: 'pm:test' })
  const iterNumber = Number(create.message?.match(/#(\d+)/)?.[1])
  expect(iterNumber).toBeGreaterThan(0)
  await post('designate_current_iteration', {
    iteration_number: iterNumber,
    agent: 'pm:test'
  })
  return iterNumber
}

export const launchIsolatedWithSeededIteration = async (
  opts: LaunchOpts & { themeName?: string } = {}
): Promise<LaunchResult & { win: Page; currentIterationNumber: number }> => {
  const { themeName = 'SeededIteration', ...launchOpts } = opts
  const result = await launchIsolatedWithWindow(launchOpts)
  const currentIterationNumber = await seedCurrentIterationViaTestApi(
    result.backendPort,
    result.authToken,
    themeName
  )
  await result.win.reload()
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  return { ...result, currentIterationNumber }
}

export const launchWithMockGhAndSeededIteration = async (
  opts: LaunchOpts & { themeName?: string } = {}
): Promise<MockGhLaunchResult & { currentIterationNumber: number }> => {
  const { themeName = 'SeededIteration', ...launchOpts } = opts
  const result = await launchWithMockGh(launchOpts)
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  const currentIterationNumber = await seedCurrentIterationViaRpc(result.rpc, themeName)
  await result.win.reload()
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  return { ...result, currentIterationNumber }
}
