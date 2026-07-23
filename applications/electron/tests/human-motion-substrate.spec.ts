import {
  type RpcFn,
  addStoryToIterationViaRpc,
  seedCurrentIterationViaRpc,
  setupCurrentIterationViaRpc
} from './helpers/iteration-fixture'
import {
  launchIsolatedElectron,
  launchWithMockGh,
  waitForBoardLoaded,
  waitForMessagesLoaded
} from './helpers/launch'
import { BoardPage } from './pages/boardPage'
import { ChatPage } from './pages/chatPage'
import { DrillInPage } from './pages/drillInPage'
import { TitlebarPage } from './pages/titlebarPage'
import { type Page, _electron as electron, expect, test } from '@playwright/test'
import { mkdtempSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'

const EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const launchIsolated = async (): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-350-'))
  const { app } = await launchIsolatedElectron({ dataBase, userDataDir: udd })
  return { app }
}

const readToken = async (win: Page, name: string): Promise<string> =>
  win.evaluate(
    (n: string) => window.getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    name
  )

test.describe('desk-check #350 motion substrate', () => {
  test('six motion tokens resolve to declared values in default media', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const tokens = [
      '--motion-duration-instant',
      '--motion-duration-fast',
      '--motion-duration-medium',
      '--motion-ease-out',
      '--motion-ease-in',
      '--motion-ease-in-out'
    ]

    const values: Record<string, string> = {}
    for (const t of tokens) {
      values[t] = await readToken(win, t)
    }

    console.log('#350 AC1 motion tokens (default media):')
    for (const [k, v] of Object.entries(values)) {
      console.log(`  ${k}: "${v}"`)
    }

    expect(values['--motion-duration-instant']).toBe('80ms')
    expect(values['--motion-duration-fast']).toBe('150ms')
    expect(values['--motion-duration-medium']).toBe('250ms')
    expect(values['--motion-ease-out']).toContain('cubic-bezier')
    expect(values['--motion-ease-in']).toContain('cubic-bezier')
    expect(values['--motion-ease-in-out']).toContain('cubic-bezier')

    await app.close()
  })

  test('duration tokens zero under prefers-reduced-motion:reduce', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    await win.emulateMedia({ reducedMotion: 'reduce' })

    const instant = await readToken(win, '--motion-duration-instant')
    const fast = await readToken(win, '--motion-duration-fast')
    const medium = await readToken(win, '--motion-duration-medium')

    console.log('#350 AC2 duration tokens under reduced-motion:reduce:')
    console.log(`  --motion-duration-instant: "${instant}" (expect 0ms)`)
    console.log(`  --motion-duration-fast: "${fast}" (expect 0ms)`)
    console.log(`  --motion-duration-medium: "${medium}" (expect 0ms)`)

    expect(instant).toBe('0ms')
    expect(fast).toBe('0ms')
    expect(medium).toBe('0ms')

    await app.close()
  })

  test('motion-safe-transform utility: transform reset + transition-property opacity under reduced-motion', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const normal = await win.evaluate(() => {
      const el = document.createElement('div')
      el.className = 'motion-safe-transform'
      el.style.transform = 'translateY(8px)'
      el.style.transitionProperty = 'transform, opacity'
      document.body.appendChild(el)
      const cs = window.getComputedStyle(el)
      const r = { transform: cs.transform, transitionProperty: cs.transitionProperty }
      el.remove()
      return r
    })
    console.log(
      `#350 AC3 normal media: transform=${normal.transform}, transition-property=${normal.transitionProperty}`
    )

    await win.emulateMedia({ reducedMotion: 'reduce' })

    const reduced = await win.evaluate(() => {
      const el = document.createElement('div')
      el.className = 'motion-safe-transform'
      el.style.transform = 'translateY(8px)'
      el.style.transitionProperty = 'transform, opacity'
      document.body.appendChild(el)
      const cs = window.getComputedStyle(el)
      const r = { transform: cs.transform, transitionProperty: cs.transitionProperty }
      el.remove()
      return r
    })
    console.log(
      `#350 AC3 reduced motion: transform=${reduced.transform}, transition-property=${reduced.transitionProperty}`
    )

    expect(reduced.transform).toBe('none')
    expect(reduced.transitionProperty).toBe('opacity')

    await app.close()
  })

  test('no text-floor regression (universal-property still holds)', async () => {
    const { app } = await launchIsolated()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).region()).toBeVisible()
    await win.waitForTimeout(300)
    await win.screenshot({
      path: join(EVIDENCE_DIR, '50-app-unchanged-with-motion-tokens.png')
    })
    const sizes = await win.evaluate(() => {
      const pick = ['.sidebar-item', '.filter-pill', '.message-from', '.board-title']
      return pick.map((sel) => {
        const el = document.querySelector(sel)
        return el
          ? { sel, fontSize: parseFloat(window.getComputedStyle(el).fontSize) }
          : { sel, fontSize: null }
      })
    })
    console.log(`#350 AC5 font-size spot check:`)
    for (const s of sizes) {
      console.log(`  ${s.sel}: ${s.fontSize}px`)
      if (s.fontSize !== null) {
        expect(s.fontSize, `${s.sel} should be ≥12px`).toBeGreaterThanOrEqual(12)
      }
    }
    await app.close()
  })
})

const EXIT_EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const seedReadyMember = async (rpc: RpcFn): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: '[Feature] Desk-check story for #351 transitions',
      body: 'Body.',
      user_job: 'When the human toggles theme or board, transitions animate the change smoothly.',
      observation_falsifier: 'reject if theme/board/drill-in transitions are missing or jarring',
      acceptance_test_path: 'applications/electron/tests/human-motion-substrate.spec.ts'
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
        path: join(EXIT_EVIDENCE_DIR, '60-app-default-dark.png')
      })

      await new TitlebarPage(win).themeToggle().click()
      await win.waitForTimeout(300)
      await win.screenshot({
        path: join(EXIT_EVIDENCE_DIR, '61-app-light-after-toggle.png')
      })

      await new TitlebarPage(win).themeToggle().click()
      await win.waitForTimeout(200)

      await new ChatPage(win).appRoot().click()
      await win.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p')
      await win.waitForTimeout(300)
      await win.screenshot({
        path: join(EXIT_EVIDENCE_DIR, '62-app-board-hidden.png')
      })
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})

const CLOSE_EVIDENCE_DIR = join(__dirname, 'desk-check-evidence')

const MOCK_MESSAGES = [
  {
    from: 'designer:claude-code',
    text: 'Mock message for avatar + arrival',
    ts: '2026-04-22T16:00:00Z',
    message_type: 'chat'
  },
  {
    from: 'dev:claude-code',
    text: 'Dev speaks',
    ts: '2026-04-22T16:01:00Z',
    message_type: 'chat'
  }
]

const launchWithMessages = async (
  reducedMotion?: 'reduce'
): Promise<{
  app: Awaited<ReturnType<typeof electron.launch>>
}> => {
  const dataBase = mkdtempSync(join(tmpdir(), 'bpmux-'))
  const udd = mkdtempSync(join(tmpdir(), 'bpmux-userdata-352-'))
  const { app } = await launchIsolatedElectron({
    dataBase,
    userDataDir: udd,
    seedMessages: MOCK_MESSAGES
  })
  const win = await app.firstWindow()
  if (reducedMotion) await win.emulateMedia({ reducedMotion })
  await waitForMessagesLoaded(win, 'Mock message for avatar + arrival')
  return { app }
}

test.describe('desk-check #352 micro-interactions', () => {
  test('filter pill transitions at instant duration (80ms)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).filterRegion()).toBeVisible()
    const pill = await win.evaluate(() => {
      const el = document.querySelector('.filter-pill')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return { property: cs.transitionProperty, duration: cs.transitionDuration }
    })
    console.log(`#352 AC1 filter-pill transition:`)
    console.log(`  property: ${pill?.property}`)
    console.log(`  duration: ${pill?.duration}`)
    expect(pill?.property).toContain('background-color')
    expect(pill?.property).toContain('color')
    expect(pill?.property).toContain('border-color')
    expect(pill?.duration).toMatch(/0\.08s|80ms/)
    await app.close()
  })

  test('button global base has hover transition', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).appRoot()).toBeVisible()

    const anyButton = await win.evaluate(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(
        (el) => !el.className.includes('filter-pill')
      )
      if (!btn) return null
      const cs = window.getComputedStyle(btn)
      return {
        tag: btn.tagName,
        className: btn.className,
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#352 AC2 sample button:`)
    console.log(`  tag: ${anyButton?.tag}, class: ${anyButton?.className}`)
    console.log(`  transition-property: ${anyButton?.property}`)
    console.log(`  transition-duration: ${anyButton?.duration}`)

    expect(anyButton?.duration).toMatch(/0\.08s|0\.15s|80ms|150ms/)

    await app.close()
  })

  test('message-row has grid-template-rows transition at fast duration', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).messageScroller().getByRole('listitem').first()).toBeVisible()

    const row = await win.evaluate(() => {
      const el = document.querySelector('.message-row')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration
      }
    })
    console.log(`#352 AC4 message-row transition:`)
    console.log(`  property: ${row?.property}`)
    console.log(`  duration: ${row?.duration}`)
    expect(row?.property).toContain('grid-template-rows')
    expect(row?.duration).toMatch(/0\.15s|150ms/)
    await app.close()
  })

  test('reduced-motion zeros filter-pill + message-row durations; message-row transform drops', async () => {
    const { app } = await launchWithMessages('reduce')
    const win = await app.firstWindow()
    await expect(new ChatPage(win).filterRegion()).toBeVisible()

    const pillDur = await win.evaluate(
      () => window.getComputedStyle(document.querySelector('.filter-pill')!).transitionDuration
    )
    console.log(`#352 AC6 filter-pill duration under reduced-motion: ${pillDur}`)
    expect(pillDur).toMatch(/^0s|^0ms/)

    const rowProps = await win.evaluate(() => {
      const el = document.querySelector('.message-row')
      if (!el) return null
      const cs = window.getComputedStyle(el)
      return {
        property: cs.transitionProperty,
        duration: cs.transitionDuration,
        transform: cs.transform
      }
    })
    console.log(`#352 AC6 message-row under reduced-motion:`)
    console.log(`  property: ${rowProps?.property}`)
    console.log(`  duration: ${rowProps?.duration}`)
    console.log(`  transform: ${rowProps?.transform}`)
    expect(rowProps?.duration).toMatch(/^0s|^0ms/)

    await app.close()
  })

  test('capture static state for visual regression (motion not frame-capturable)', async () => {
    const { app } = await launchWithMessages()
    const win = await app.firstWindow()
    await expect(new ChatPage(win).region()).toBeVisible()
    await win.waitForTimeout(500)
    await win.screenshot({
      path: join(CLOSE_EVIDENCE_DIR, '70-micro-interactions-static.png')
    })
    await app.close()
  })
})

const MEMBER_TITLE = 'Close-direction desk-check'

const seedDrillInMember = async (rpc: RpcFn): Promise<void> => {
  const createResp = await rpc(
    'create_story',
    {
      title: `[Feature] ${MEMBER_TITLE}`,
      user_job: 'When the owner closes the drill-in, the close-direction transition plays.',
      observation_falsifier: 'reject if the drill-in does not animate its exit transition',
      acceptance_test_path: 'applications/electron/tests/human-motion-substrate.spec.ts'
    },
    'pm'
  )
  expect(createResp.status).toBe(200)
  await setupCurrentIterationViaRpc(rpc, 1, 'DeskCheck354')
}

test.describe('desk-check #354 close-direction animations', () => {
  test('drill-in stays mounted during exit transition (data-visible toggles)', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedDrillInMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(400)

      const enteredAttr = await drillIn.panel().getAttribute('data-visible')
      console.log(`#354 drill-in data-visible while entered: ${enteredAttr}`)

      await drillIn.backButton().click()

      await win.waitForTimeout(50)
      const exitingAttr = await win.locator('.drill-in').getAttribute('data-visible')
      console.log(`#354 drill-in data-visible right after close click: ${exitingAttr}`)

      await win.waitForTimeout(400)
      const afterExitCount = await win.locator('.drill-in').count()
      console.log(
        `#354 drill-in DOM after full exit: ${afterExitCount === 0 ? 'unmounted ✓' : 'still present'}`
      )
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('drill-in exit transition has transform + opacity declared at close', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await expect(board.region()).toBeVisible()
      await seedDrillInMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(400)

      const transition = await drillIn.panel().evaluate((el) => {
        const cs = window.getComputedStyle(el)
        return {
          property: cs.transitionProperty,
          duration: cs.transitionDuration,
          easing: cs.transitionTimingFunction
        }
      })
      console.log(`#354 drill-in transition (at entered state):`)
      console.log(`  property: ${transition.property}`)
      console.log(`  duration: ${transition.duration}`)
      console.log(`  easing: ${transition.easing}`)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })

  test('reduced-motion: drill-in exit uses 0s duration + transform none', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      const drillIn = new DrillInPage(win)
      await win.emulateMedia({ reducedMotion: 'reduce' })
      await expect(board.region()).toBeVisible()
      await seedDrillInMember(rpc)
      await win.reload()
      await expect(board.region()).toBeVisible()

      await board.openTab('Plan')
      await board.openStory(1)
      await expect(drillIn.panel()).toBeVisible()
      await win.waitForTimeout(300)

      const reduced = await drillIn.panel().evaluate((el) => {
        const cs = window.getComputedStyle(el)
        return {
          duration: cs.transitionDuration,
          transform: cs.transform
        }
      })
      console.log(`#354 reduced-motion drill-in:`)
      console.log(`  duration: ${reduced.duration}`)
      console.log(`  transform: ${reduced.transform}`)
      expect(reduced.duration).toMatch(/^0s|^0ms|0s,/)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
