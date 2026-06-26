import { test, expect } from '@playwright/test'
import { join } from 'path'
import { tmpdir } from 'os'
import type { Page } from '@playwright/test'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import type { RpcFn } from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const OUTCOME_TITLE = 'You see what is coming, and whether it is ready'
const CRUX_TITLE = 'Iteration Crux'
const LYRA_TITLE = 'Iteration Lyra'

const numberFromMessage = async (resp: Response): Promise<number> => {
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const n = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(n).toBeGreaterThan(0)
  return n
}

const beforeContent = (win: Page, selector: string): Promise<string> =>
  win.evaluate((sel) => {
    const el = document.querySelector(sel)
    return el === null ? '' : getComputedStyle(el, '::before').content
  }, selector)

interface BoardStorySnapshot {
  number: number
  iteration_order: number | null
  readiness: { drafted: boolean } | null
  outcome_membership: { outcome_number: number; position: number } | null
}

const boardStory = async (
  result: MockGhLaunchResult,
  storyNumber: number
): Promise<BoardStorySnapshot | undefined> => {
  const resp = await fetch(`http://127.0.0.1:${result.backendPort}/api/board`, {
    headers: { Authorization: `Bearer ${result.authToken}` }
  })
  expect(resp.status).toBe(200)
  const board = (await resp.json()) as { stories: BoardStorySnapshot[] }
  return board.stories.find((s) => s.number === storyNumber)
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

const waitForIterationSettled = async (
  result: MockGhLaunchResult,
  iterationNumber: number,
  predicate: (s: BoardStorySnapshot) => boolean
): Promise<void> => {
  const deadline = Date.now() + 20_000
  let hits = 0
  while (Date.now() < deadline) {
    const story = await boardStory(result, iterationNumber)
    if (story !== undefined && predicate(story)) {
      hits += 1
      if (hits >= 3) return
    } else {
      hits = 0
    }
    await sleep(300)
  }
  throw new Error(`iteration #${iterationNumber} did not settle within 20s`)
}

const createMember = async (rpc: RpcFn, label: string): Promise<number> => {
  const resp = await rpc(
    'create_story',
    {
      title: `[Feature] ${label}`,
      user_job: `Member ${label} so the iteration counts as stories-ordered.`,
      observation_falsifier: 'reject if the member does not order the iteration',
      acceptance_test_path: 'applications/electron/tests/roadmap-outcome-drill-in.spec.ts'
    },
    'pm'
  )
  return numberFromMessage(resp)
}

const seedOutcomeRoadmap = async (result: MockGhLaunchResult): Promise<void> => {
  const { rpc } = result
  const cruxMember = await createMember(rpc, 'crux member')

  const outcome = await numberFromMessage(
    await rpc(
      'create_outcome',
      {
        title: OUTCOME_TITLE,
        horizon: 'now',
        body: '**Benefit:** you steer the next iterations before convening\n**Confidence:** high'
      },
      'pm'
    )
  )

  const crux = await numberFromMessage(await rpc('create_iteration', { name: 'Crux' }, 'pm'))
  await waitForIterationSettled(result, crux, (s) => s.iteration_order !== null)
  const lyra = await numberFromMessage(await rpc('create_iteration', { name: 'Lyra' }, 'pm'))
  await waitForIterationSettled(result, lyra, (s) => s.iteration_order !== null)

  const mapCrux = await rpc(
    'map_iteration_to_outcome',
    { outcome_number: outcome, iteration_number: crux, position: 1 },
    'pm'
  )
  expect(mapCrux.status).toBe(200)
  const mapLyra = await rpc(
    'map_iteration_to_outcome',
    { outcome_number: outcome, iteration_number: lyra, position: 2 },
    'pm'
  )
  expect(mapLyra.status).toBe(200)

  const addMember = await rpc(
    'add_story_to_iteration',
    { story_number: cruxMember, iteration_number: crux, position: 1 },
    'pm'
  )
  expect(addMember.status).toBe(200)

  const blankLyra = await rpc('edit_story', { number: lyra, body: ' ' }, 'pm')
  expect(blankLyra.status).toBe(200)

  await waitForIterationSettled(
    result,
    crux,
    (s) => s.readiness?.drafted === true && s.outcome_membership?.outcome_number === outcome
  )
  await waitForIterationSettled(
    result,
    lyra,
    (s) => s.readiness?.drafted === false && s.outcome_membership?.outcome_number === outcome
  )
}

test.describe('#1737 roadmap outcome drill-in — focus an outcome → its iterations, in order', () => {
  test('focusing an outcome reveals the iterations building it (in order) with convene-checks + the CONVENE? banner', async () => {
    test.setTimeout(90_000)
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    const consoleErrors: string[] = []
    win.on('console', (m) => {
      if (m.type() === 'error') consoleErrors.push(m.text())
    })
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedOutcomeRoadmap(result)
      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      await roadmap.selectView('Roadmap')
      await expect(roadmap.panel()).toBeVisible()

      await expect(roadmap.outcome(OUTCOME_TITLE)).toBeVisible()
      await expect(roadmap.drill()).toHaveCount(0)

      consoleErrors.length = 0

      await roadmap.outcome(OUTCOME_TITLE).click()

      await expect(roadmap.drill()).toBeVisible()

      const names = await roadmap.iterationOrder().allTextContents()
      expect(names, 'the iterations building the outcome render IN ORDER').toEqual([
        CRUX_TITLE,
        LYRA_TITLE
      ])

      const convene = roadmap.convene()
      await expect(convene).toContainText('CONVENE?')
      await expect(convene).toContainText('Crux')
      await expect(convene).toContainText('designer-review')

      await expect(roadmap.checks(CRUX_TITLE)).not.toHaveCount(0)
      await expect(roadmap.checks(LYRA_TITLE)).toHaveCount(0)
      await expect(roadmap.iterationRow(LYRA_TITLE)).toHaveAttribute('data-state', 'not-started')

      await expect
        .poll(() => beforeContent(win, '.roadmap-drill-check[data-state="met"]'))
        .toContain('✓')
      await expect
        .poll(() => beforeContent(win, '.roadmap-drill-check[data-state="gap"]'))
        .toContain('✗')

      expect(consoleErrors, 'no console errors while drilling an outcome').toEqual([])

      const shotPath = join(tmpdir(), 'bpmux-1737-drill.png')
      await win.screenshot({ path: shotPath, fullPage: false })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
