import { test, expect } from '@playwright/test'
import { launchWithMockGh, type MockGhLaunchResult } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import type { RpcFn } from './helpers/iteration-fixture'
import type { Page } from '@playwright/test'
import { BoardPage } from './pages/boardPage'
import { RoadmapPage } from './pages/roadmapPage'

const iterationOrderOf = async (
  result: MockGhLaunchResult,
  iterationNumber: number
): Promise<number | null> => {
  const resp = await fetch(`http://127.0.0.1:${result.backendPort}/api/iterations`, {
    headers: { Authorization: `Bearer ${result.authToken}` }
  })
  expect(resp.status).toBe(200)
  const body = (await resp.json()) as {
    iterations: { number: number; iteration_order: number | null }[]
  }
  return body.iterations.find((s) => s.number === iterationNumber)?.iteration_order ?? null
}

const sleep = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms))

const waitForStableIterationOrder = async (
  result: MockGhLaunchResult,
  iterationNumber: number
): Promise<number> => {
  const deadline = Date.now() + 20_000
  let stableValue: number | null = null
  let stableHits = 0
  while (Date.now() < deadline) {
    const order = await iterationOrderOf(result, iterationNumber)
    if (order !== null && order === stableValue) {
      stableHits += 1
      if (stableHits >= 3) return order
    } else {
      stableValue = order
      stableHits = order === null ? 0 : 1
    }
    await sleep(300)
  }
  throw new Error(`iteration #${iterationNumber} order did not settle within 20s`)
}

const numberFromMessage = async (resp: Response): Promise<number> => {
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const n = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(n).toBeGreaterThan(0)
  return n
}

const createMember = async (rpc: RpcFn, label: string): Promise<number> => {
  const resp = await rpc(
    'create_story',
    {
      title: `[Feature] ${label}`,
      user_job: `Member ${label} so the current iteration can drain toward close-approach.`,
      observation_falsifier: 'reject if the member does not land in the current iteration column',
      acceptance_test_path: 'applications/electron/tests/close-approach-nudge-1705.spec.ts'
    },
    'pm'
  )
  return numberFromMessage(resp)
}

const createIteration = async (rpc: RpcFn, name: string): Promise<number> => {
  const resp = await rpc('create_iteration', { name }, 'pm')
  return numberFromMessage(resp)
}

const startMember = async (rpc: RpcFn, number: number): Promise<void> => {
  const resp = await rpc('start_story', {
    number,
    pickup_paraphrase: 'Driving the member into the current iteration so it can drain.',
    falsifier_reframing: 'reject if start_story does not move the member to in-progress'
  })
  expect(resp.status).toBe(200)
}

const deliverMember = async (rpc: RpcFn, number: number): Promise<void> => {
  await startMember(rpc, number)
  const deliverResp = await rpc('deliver_story', {
    number,
    verified_observation: 'Drove deliver so the member lands in the Acceptance column.'
  })
  expect(deliverResp.status).toBe(200)
  await seedTeamVerdicts(rpc, number, ['designer'])
  const passResp = await rpc(
    'pass_desk_check',
    { number, acceptor: 'team', would_file_bug: false },
    'designer'
  )
  expect(passResp.status).toBe(200)
}

interface NudgeTopology {
  nextNumber: number
  nextName: string
}

const seedCloseApproach = async (
  result: MockGhLaunchResult,
  makeNextReady: boolean
): Promise<NudgeTopology> => {
  const { rpc } = result
  const currentMembers: number[] = []
  for (let i = 1; i <= 4; i += 1) {
    currentMembers.push(await createMember(rpc, `current member ${i}`))
  }
  const nextMember = await createMember(rpc, 'next member')

  const currentName = 'Lupus'
  const nextName = 'Fornax'
  const currentIter = await createIteration(rpc, currentName)
  const currentOrder = await waitForStableIterationOrder(result, currentIter)
  const nextIter = await createIteration(rpc, nextName)
  const nextOrder = await waitForStableIterationOrder(result, nextIter)
  expect(nextOrder, 'the next iteration sorts after the current one').toBeGreaterThan(currentOrder)

  const designateResp = await rpc(
    'designate_current_iteration',
    { iteration_number: currentIter },
    'pm'
  )
  expect(designateResp.status).toBe(200)

  for (let i = 0; i < currentMembers.length; i += 1) {
    const addResp = await rpc(
      'add_story_to_iteration',
      { story_id: currentMembers[i], iteration_number: currentIter, position: i + 1 },
      'pm'
    )
    expect(addResp.status).toBe(200)
  }

  const addNextResp = await rpc(
    'add_story_to_iteration',
    { story_id: nextMember, iteration_number: nextIter, position: 1 },
    'pm'
  )
  expect(addNextResp.status).toBe(200)

  await deliverMember(rpc, currentMembers[0])
  await deliverMember(rpc, currentMembers[1])
  await deliverMember(rpc, currentMembers[2])
  await startMember(rpc, currentMembers[3])

  if (makeNextReady) {
    const reviewResp = await rpc(
      'close_pre_pickup_review',
      { iteration_number: nextIter },
      'designer'
    )
    expect(reviewResp.status).toBe(200)
    const devOkResp = await rpc('dev_ok_iteration', { iteration_number: nextIter }, 'dev')
    expect(devOkResp.status).toBe(200)
  }

  return { nextNumber: nextIter, nextName }
}

type MotionTrace = { minOpacity: number | null; maxAnims: number; samples: number }

const traceFromFirstAppearance = (win: Page): Promise<MotionTrace> =>
  win.evaluate(
    () =>
      new Promise<MotionTrace>((resolve) => {
        const opacities: number[] = []
        const anims: number[] = []
        let postFrames = 0
        let totalFrames = 0
        const tick = (): void => {
          const el = document.querySelector('.close-approach-nudge')
          if (el) {
            opacities.push(parseFloat(getComputedStyle(el).opacity))
            anims.push(el.getAnimations().length)
            postFrames += 1
          }
          totalFrames += 1
          const waitingForAppearance = opacities.length === 0 && totalFrames < 240
          const samplingAfterAppearance = opacities.length > 0 && postFrames < 30
          if (waitingForAppearance || samplingAfterAppearance) requestAnimationFrame(tick)
          else
            resolve({
              minOpacity: opacities.length ? Math.min(...opacities) : null,
              maxAnims: anims.length ? Math.max(...anims) : 0,
              samples: opacities.length
            })
        }
        requestAnimationFrame(tick)
      })
  )

test.describe('#1705 close-approach readiness nudge', () => {
  test('close-approach + next-not-ready: the strip renders in-frame (owner-visible, warn-not-block) + ◐ tab-badge, and the ENTER slide PLAYS (opacity 0→1, getAnimations live)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      const { nextName } = await seedCloseApproach(result, false)
      await win.reload()
      await expect(board.region()).toBeVisible()

      const trace = traceFromFirstAppearance(win)
      const strip = board.closeApproachNudge()
      await expect(strip).toBeVisible()
      await expect(strip).toContainText(nextName)
      await expect(strip).toContainText('designer-review')
      await expect(board.roadmapAttn()).toBeVisible()
      await expect(board.tabBar()).toBeVisible()

      const t = await trace
      expect(t.samples, 'strip was observed from its first frame').toBeGreaterThan(0)
      expect(t.maxAnims, 'a CSS animation/transition ran on the strip (it plays)').toBeGreaterThan(
        0
      )
      expect(
        t.minOpacity,
        'opacity rose from <1 — the slide-fade entered, not popped'
      ).toBeLessThan(1)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('dismiss collapses the strip (the EXIT plays) but the ◐ tab-badge survives (recognition-over-recall)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCloseApproach(result, false)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.closeApproachNudge()).toBeVisible()
      await board.closeApproachDismiss().click()
      const exitAnims = await win.evaluate(
        () =>
          new Promise<number>((resolve) => {
            let max = 0
            let frames = 0
            const tick = (): void => {
              const el = document.querySelector('.close-approach-nudge')
              if (el) max = Math.max(max, el.getAnimations().length)
              frames += 1
              if (frames < 30) requestAnimationFrame(tick)
              else resolve(max)
            }
            requestAnimationFrame(tick)
          })
      )
      expect(exitAnims, 'the collapse transition ran (exit plays)').toBeGreaterThan(0)
      await expect(board.closeApproachNudge()).toHaveAttribute('data-dismissed', 'true')
      await expect(board.roadmapAttn()).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('next-ready: no strip and no tab-badge (signal-honesty — only an un-ready NEXT trips it)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCloseApproach(result, true)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.tabBar()).toBeVisible()
      await expect(board.closeApproachNudge()).toHaveCount(0)
      await expect(board.roadmapAttn()).toHaveCount(0)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('the strip is board-list-scoped — rendered INSIDE the board panel, not over-rendering past the board-list container', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCloseApproach(result, false)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.closeApproachNudge()).toBeVisible()
      await expect(board.boardScrollPanel().locator('.close-approach-nudge')).toHaveCount(1)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('off the board tab the strip is ABSENT from the DOM (conditional-render, not a hidden attribute) — the ◐ tab-badge carries the cue', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      const roadmap = new RoadmapPage(win)
      await expect(board.region()).toBeVisible()
      await seedCloseApproach(result, false)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await expect(board.closeApproachNudge()).toBeVisible()
      await roadmap.selectView('Roadmap')
      await expect(board.closeApproachNudgeRaw()).toHaveCount(0)
      await expect(board.roadmapAttn()).toBeVisible()
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('reduced-motion: the strip is present but does NOT animate (instant, no slide)', async () => {
    const result = await launchWithMockGh()
    const { app, win, mockGh } = result
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()
      await seedCloseApproach(result, false)
      await win.emulateMedia({ reducedMotion: 'reduce' })
      await win.reload()
      await expect(board.region()).toBeVisible()

      const trace = traceFromFirstAppearance(win)
      await expect(board.closeApproachNudge()).toBeVisible()
      const t = await trace
      expect(t.maxAnims, 'reduced-motion ran NO animation (instant present)').toBe(0)
      const duration = await win.evaluate(
        () =>
          getComputedStyle(document.querySelector('.close-approach-nudge') as Element)
            .transitionDuration
      )
      expect(duration).toMatch(/^0s|^0ms|0s,/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
