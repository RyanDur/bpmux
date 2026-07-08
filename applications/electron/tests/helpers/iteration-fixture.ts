import { expect } from '@playwright/test'
import { launchIsolatedWithWindow, launchWithMockGh } from './launch'
import type { LaunchOpts, LaunchResult, MockGhLaunchResult } from './launch'
import type { Page } from '@playwright/test'

export type RpcFn = (tool: string, body: unknown, agent?: string) => Promise<Response>

export const seedCurrentIterationViaRpc = async (rpc: RpcFn, name: string): Promise<number> => {
  const iterResp = await rpc('create_iteration', { name }, 'pm')
  expect(iterResp.status).toBe(200)
  const iterJson = (await iterResp.json()) as { message?: string }
  const iterationId = Number(iterJson.message?.match(/#(\d+)/)?.[1])
  expect(iterationId).toBeGreaterThan(0)

  await expect(async () => {
    const designateResp = await rpc(
      'designate_current_iteration',
      { iteration_id: iterationId },
      'pm'
    )
    expect(designateResp.status).toBe(200)
  }).toPass({ timeout: 10_000 })

  return iterationId
}

export const addStoryToIterationViaRpc = async (
  rpc: RpcFn,
  storyId: number,
  iterationId: number,
  position: number
): Promise<void> => {
  const addResp = await rpc(
    'add_story_to_iteration',
    { story_id: storyId, iteration_id: iterationId, position },
    'pm'
  )
  expect(addResp.status).toBe(200)
}

export const setupCurrentIterationViaRpc = async (
  rpc: RpcFn,
  storyId: number,
  name: string
): Promise<number> => {
  const iterationId = await seedCurrentIterationViaRpc(rpc, name)
  await addStoryToIterationViaRpc(rpc, storyId, iterationId, 1)
  return iterationId
}

export const seedCurrentIterationViaTestApi = async (
  backendPort: number,
  authToken: string,
  name: string
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
  const create = await post('create_iteration', { name, agent: 'pm:test' })
  const iterationId = Number(create.message?.match(/#(\d+)/)?.[1])
  expect(iterationId).toBeGreaterThan(0)
  await post('designate_current_iteration', {
    iteration_id: iterationId,
    agent: 'pm:test'
  })
  return iterationId
}

export const launchIsolatedWithSeededIteration = async (
  opts: LaunchOpts & { name?: string } = {}
): Promise<LaunchResult & { win: Page; currentIterationId: number }> => {
  const { name = 'SeededIteration', ...launchOpts } = opts
  const result = await launchIsolatedWithWindow(launchOpts)
  const currentIterationId = await seedCurrentIterationViaTestApi(
    result.backendPort,
    result.authToken,
    name
  )
  await result.win.reload()
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  return { ...result, currentIterationId }
}

export const launchWithMockGhAndSeededIteration = async (
  opts: LaunchOpts & { name?: string } = {}
): Promise<MockGhLaunchResult & { currentIterationId: number }> => {
  const { name = 'SeededIteration', ...launchOpts } = opts
  const result = await launchWithMockGh(launchOpts)
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  const currentIterationId = await seedCurrentIterationViaRpc(result.rpc, name)
  await result.win.reload()
  await expect(result.win.locator('[data-test="board-region"]')).toBeVisible()
  return { ...result, currentIterationId }
}
