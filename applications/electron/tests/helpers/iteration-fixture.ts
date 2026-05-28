import { expect } from '@playwright/test'

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

export const setupCurrentIterationViaRpc = async (
  rpc: RpcFn,
  storyNumber: number,
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

  const addResp = await rpc(
    'add_story_to_iteration',
    { story_number: storyNumber, iteration_number: iterNumber, position: 1 },
    'pm'
  )
  expect(addResp.status).toBe(200)

  return iterNumber
}
