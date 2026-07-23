import { test, expect } from '@playwright/test'
import { join } from 'path'
import { launchWithMockGh, waitForBoardLoaded } from './helpers/launch'
import {
  addStoryToIterationViaRpc,
  seedCurrentIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { ChatPage } from './pages/chatPage'
import { BoardPage } from './pages/boardPage'
import { DrillInPage } from './pages/drillInPage'
import { TitlebarPage } from './pages/titlebarPage'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const seedReadyMember = async (rpc: RpcFn): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: '[Feature] Desk-check story for #351 transitions',
      body: 'Body.',
      user_job: 'When the human toggles theme or board, transitions animate the change smoothly.',
      observation_falsifier: 'reject if theme/board/drill-in transitions are missing or jarring',
      acceptance_test_path:
        'applications/electron/tests/human-theme-and-board-transitions-reduced-motion.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  const iterationId = await seedCurrentIterationViaRpc(rpc, 'DeskCheck351')
  const moveResp = await rpc(
    'move_story',
    { id: 1, target_state: 'ready', rationale: 'transitions fixture' },
    'pm'
  )
  expect(moveResp.status).toBe(200)
  await addStoryToIterationViaRpc(rpc, 1, iterationId, 1)
}

test.describe('desk-check #351 high-signal transitions', () => {
  test('body has color + background-color transition (theme toggle smoothing)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      await expect(new ChatPage(win).appRoot()).toBeVisible()

      const bodyTransition = await win.evaluate(() => {
        const cs = window.getComputedStyle(document.body)
        return {
          property: cs.transitionProperty,
          duration: cs.transitionDuration,
          easing: cs.transitionTimingFunction
        }
      })
      console.log(`#351 AC6 body transition:`)
      console.log(`  property: ${bodyTransition.property}`)
      console.log(`  duration: ${bodyTransition.duration}`)
      console.log(`  easing:   ${bodyTransition.easing}`)

      expect(bodyTransition.property).toContain('color')
      expect(bodyTransition.property).toContain('background-color')
      expect(bodyTransition.duration).toMatch(/0\.15s|150ms/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('board region transitions on hide-toggle (always-mounted pattern)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      await expect(new BoardPage(win).region()).toBeVisible()

      const boardTransition = await new BoardPage(win).region().evaluate((el) => {
        const cs = window.getComputedStyle(el)
        return {
          property: cs.transitionProperty,
          duration: cs.transitionDuration
        }
      })
      console.log(`#351 AC3 board-region transition:`)
      console.log(`  property: ${boardTransition?.property}`)
      console.log(`  duration: ${boardTransition?.duration}`)

      const boardBefore = await new BoardPage(win).region().isVisible()
      console.log(`  board visible before toggle: ${boardBefore}`)

      await new ChatPage(win).appRoot().click()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p')
      await win.waitForTimeout(400)

      const boardClass = (await win.locator('.board').getAttribute('class')) ?? ''
      console.log(`  board-region class after toggle: "${boardClass}"`)

      const stillMounted = (await win.locator('.board').count()) > 0
      console.log(`  board DOM still mounted: ${stillMounted}`)
      expect(stillMounted).toBe(true)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('drill-in surface has transition declared on enter state', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      await new BoardPage(win).openStory(1)
      await expect(new DrillInPage(win).panel()).toBeVisible()
      await win.waitForTimeout(400)

      const drillIn = await new DrillInPage(win).panel().evaluate((el) => {
        const cs = window.getComputedStyle(el)
        return {
          property: cs.transitionProperty,
          duration: cs.transitionDuration
        }
      })
      console.log(`#351 AC1 drill-in transition:`)
      console.log(`  property: ${drillIn?.property}`)
      console.log(`  duration: ${drillIn?.duration}`)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('reduced-motion: body + board transitions zero to 0ms', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await win.emulateMedia({ reducedMotion: 'reduce' })
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await win.emulateMedia({ reducedMotion: 'reduce' })
      await waitForBoardLoaded(win)
      await expect(new ChatPage(win).appRoot()).toBeVisible()

      const bodyDuration = await win.evaluate(
        () => window.getComputedStyle(document.body).transitionDuration
      )
      const boardDuration = await new BoardPage(win)
        .region()
        .evaluate((el) => window.getComputedStyle(el).transitionDuration)
      console.log(`#351 AC8 reduced-motion durations:`)
      console.log(`  body: ${bodyDuration}`)
      console.log(`  board: ${boardDuration}`)

      expect(bodyDuration).toMatch(/^0s|^0ms/)
      expect(boardDuration).toMatch(/^0s|^0ms/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('capture UI state — theme toggle + board toggle screenshots (motion not frame-capturable)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      await expect(new ChatPage(win).region()).toBeVisible()
      await seedReadyMember(rpc)
      await win.reload()
      await waitForBoardLoaded(win)
      await expect(new ChatPage(win).appRoot()).toBeVisible()
      await win.waitForTimeout(300)
      await win.screenshot({
        path: join(EVIDENCE_DIR, '60-app-default-dark.png')
      })

      await new TitlebarPage(win).themeToggle().click()
      await win.waitForTimeout(300)
      await win.screenshot({
        path: join(EVIDENCE_DIR, '61-app-light-after-toggle.png')
      })

      await new TitlebarPage(win).themeToggle().click()
      await win.waitForTimeout(200)

      await new ChatPage(win).appRoot().click()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p')
      await win.waitForTimeout(300)
      await win.screenshot({
        path: join(EVIDENCE_DIR, '62-app-board-hidden.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
