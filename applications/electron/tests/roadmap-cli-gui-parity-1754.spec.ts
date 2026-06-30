import { test, expect } from '@playwright/test'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { existsSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { launchWithMockGh } from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const execFileAsync = promisify(execFile)

const numberFromMessage = async (resp: Response): Promise<number> => {
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const n = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(n).toBeGreaterThan(0)
  return n
}

const builtBpmuxBinary = (): string => {
  const workspace = join(__dirname, '../../..')
  const release = join(workspace, 'target/release/bpmux')
  const debug = join(workspace, 'target/debug/bpmux')
  const releaseExists = existsSync(release)
  const debugExists = existsSync(debug)
  if (releaseExists && debugExists) {
    return statSync(release).mtimeMs >= statSync(debug).mtimeMs ? release : debug
  }
  return releaseExists ? release : debug
}

const bpmuxRoadmap = async (port: number, args: string[]): Promise<string> => {
  const { stdout } = await execFileAsync(builtBpmuxBinary(), ['roadmap', ...args], {
    env: { ...process.env, BPMUX_PORT: String(port), BPMUX_ROLE: 'dev' }
  })
  return stdout
}

test.describe('#1754 — an agent reading via the CLI and the owner viewing the GUI see ONE roadmap', () => {
  test('an outcome + iteration seeded once is visible in BOTH the GUI roadmap and the bpmux roadmap CLI', async () => {
    test.setTimeout(90_000)
    const result = await launchWithMockGh()
    const { app, win, mockGh, rpc, backendPort } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()

      const outcomeTitle = 'Agents reason from the shared substrate'
      const outcome = await numberFromMessage(
        await rpc(
          'create_outcome',
          { title: outcomeTitle, horizon: 'now', body: '**Benefit:** parity' },
          'pm'
        )
      )
      const buildingIteration = await numberFromMessage(
        await rpc('create_iteration', { name: 'Crux' }, 'pm')
      )
      const mapped = await rpc(
        'map_iteration_to_outcome',
        { outcome_id: outcome, iteration_id: buildingIteration, position: 1 },
        'pm'
      )
      expect(mapped.status).toBe(200)
      await win.reload()

      const ownerRoadmapView = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()
      await ownerRoadmapView.selectView('Roadmap')
      await expect(ownerRoadmapView.panel()).toBeVisible()
      await expect(ownerRoadmapView.outcome(outcomeTitle)).toBeVisible()

      const agentSeesOutcome = await bpmuxRoadmap(backendPort, [])
      expect(agentSeesOutcome).toContain(outcomeTitle)

      const agentDrillsTheOutcome = await bpmuxRoadmap(backendPort, ['--outcome', String(outcome)])
      expect(agentDrillsTheOutcome).toContain(outcomeTitle)
      expect(agentDrillsTheOutcome).toContain('Crux')
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the agent narrows the CLI read with --state', async () => {
    test.setTimeout(90_000)
    const result = await launchWithMockGh()
    const { app, mockGh, rpc, backendPort } = result
    try {
      const needsAttentionTitle = 'Has a building iteration not yet ready'
      const plannedTitle = 'Planned with no iterations yet'
      const needsAttention = await numberFromMessage(
        await rpc(
          'create_outcome',
          { title: needsAttentionTitle, horizon: 'now', body: '**Benefit:** x' },
          'pm'
        )
      )
      const iteration = await numberFromMessage(
        await rpc('create_iteration', { name: 'Crux' }, 'pm')
      )
      const mapped = await rpc(
        'map_iteration_to_outcome',
        { outcome_id: needsAttention, iteration_id: iteration, position: 1 },
        'pm'
      )
      expect(mapped.status).toBe(200)
      await numberFromMessage(
        await rpc(
          'create_outcome',
          { title: plannedTitle, horizon: 'next', body: '**Benefit:** y' },
          'pm'
        )
      )

      await expect
        .poll(() => bpmuxRoadmap(backendPort, ['--state', 'needs-attention']), { timeout: 20_000 })
        .toContain(needsAttentionTitle)
      const filtered = await bpmuxRoadmap(backendPort, ['--state', 'needs-attention'])
      expect(filtered).not.toContain(plannedTitle)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
