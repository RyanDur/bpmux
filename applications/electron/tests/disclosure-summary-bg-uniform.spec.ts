import { test, expect } from '@playwright/test'
import { atpPath } from './helpers/atp'
import { tmpdir } from 'os'
import { join } from 'path'
import { launchWithMockGh } from './helpers/launch'
import { seedTeamVerdicts } from './helpers/seed-evidence'
import {
  seedCurrentIterationViaRpc,
  addStoryToIterationViaRpc,
  type RpcFn
} from './helpers/iteration-fixture'
import { BoardPage } from './pages/boardPage'

const SPEC_PATH = atpPath(__filename)

const createStory = async (rpc: RpcFn, title: string, parentEpic?: number): Promise<number> => {
  const body: Record<string, unknown> = {
    title,
    user_job: `Disclosure-uniformity fixture: ${title}.`,
    observation_falsifier: 'reject if this fixture row does not render in its board section',
    acceptance_test_path: SPEC_PATH
  }
  if (parentEpic !== undefined) body.parent_epic = parentEpic
  const resp = await rpc('create_story', body, 'pm')
  expect(resp.status).toBe(200)
  const json = (await resp.json()) as { message?: string }
  const number = Number(json.message?.match(/#(\d+)/)?.[1])
  expect(number).toBeGreaterThan(0)
  return number
}

const driveToAccepted = async (rpc: RpcFn, id: number): Promise<void> => {
  const startResp = await rpc('start_story', {
    id,
    pickup_paraphrase: 'Driving an epic child to accepted for the disclosure fixture.',
    falsifier_reframing: 'reject if start_story does not move story to in-progress'
  })
  expect(startResp.status).toBe(200)
  const deliverResp = await rpc('deliver_story', {
    id,
    verified_observation: 'Drove deliver so the child reaches accepted.'
  })
  expect(deliverResp.status).toBe(200)
  await seedTeamVerdicts(rpc, id, ['designer'])
  const passResp = await rpc(
    'pass_desk_check',
    { id, acceptor: 'team', would_file_bug: false },
    'designer'
  )
  expect(passResp.status).toBe(200)
  await seedTeamVerdicts(rpc, id, ['designer', 'dev'])
  const acceptResp = await rpc('accept_story', { id, would_file_bug: false }, 'pm')
  expect(acceptResp.status).toBe(200)
}

type StateBg = Record<string, string>

test.describe('#1519 disclosure-summary background uniformity (dark theme)', () => {
  test('all 4 summary toggle-headers share --bg-base resting/open and --bg-highlighted hover', async () => {
    const { app, win, mockGh, rpc } = await launchWithMockGh()
    try {
      const board = new BoardPage(win)
      await expect(board.region()).toBeVisible()

      const epicNumber = await createStory(rpc, '[Epic] disclosure-summary uniformity')
      await createStory(rpc, '[Feature] Child story under epic — ready', epicNumber)
      const acceptedChild = await createStory(
        rpc,
        '[Feature] Child story under epic — accepted',
        epicNumber
      )

      const memberNumber = await createStory(rpc, '[Feature] Member of Alpha')
      const iterationId = await seedCurrentIterationViaRpc(rpc, 'Alpha')
      await addStoryToIterationViaRpc(rpc, memberNumber, iterationId, 1)

      await createStory(rpc, '[Feature] Unscheduled loose story A')
      await createStory(rpc, '[Feature] Unscheduled loose story B')

      await driveToAccepted(rpc, acceptedChild)

      await win.evaluate(() => window.localStorage.setItem('bpmux-theme', 'dark'))
      await win.reload()
      await expect(board.region()).toBeVisible()
      await expect
        .poll(() => win.evaluate(() => document.documentElement.dataset.theme))
        .toBe('dark')

      const tokens = await win.evaluate(() => {
        const probe = (varName: string): string => {
          const el = document.createElement('span')
          el.style.backgroundColor = `var(${varName})`
          document.body.appendChild(el)
          const c = getComputedStyle(el).backgroundColor
          el.remove()
          return c
        }
        return {
          base: probe('--bg-base'),
          highlighted: probe('--bg-highlighted')
        }
      })

      await board.openTab('Plan')
      await expect(board.planContent()).toBeVisible()

      await board.openColumn('Icebox')
      await expect(board.activePlannedSection()).toBeVisible()

      type LocLike = ReturnType<typeof win.locator>
      const restRead = async (loc: LocLike): Promise<string> => {
        await win.mouse.move(0, 0)
        return loc.evaluate((el) => getComputedStyle(el as Element).backgroundColor)
      }
      const backgroundWhileHovering = async (loc: LocLike): Promise<string> => {
        await loc.hover()
        return loc.evaluate((el) => getComputedStyle(el as Element).backgroundColor)
      }
      const hoverRead = async (loc: LocLike): Promise<string> => {
        let settled = await backgroundWhileHovering(loc)
        await expect
          .poll(async () => {
            const again = await backgroundWhileHovering(loc)
            const isSettled = again === settled
            settled = again
            return isSettled
          })
          .toBe(true)
        await win.mouse.move(0, 0)
        return settled
      }
      const isOpen = (loc: LocLike): Promise<boolean> =>
        loc.evaluate((el) => (el as HTMLDetailsElement).open)
      const toggleVia = async (summary: LocLike): Promise<void> => {
        await summary.click()
        await win.mouse.move(0, 0)
      }

      const table: Record<string, StateBg> = {}

      table['board-group-summary'] = {}
      {
        const grp = board.columnToggle('Icebox')
        if ((await grp.getAttribute('aria-expanded')) !== 'true') await toggleVia(grp)
        await expect(grp).toHaveAttribute('aria-expanded', 'true')
        table['board-group-summary'].open = await restRead(grp)
        await toggleVia(grp)
        await expect(grp).toHaveAttribute('aria-expanded', 'false')
        table['board-group-summary'].collapsed = await restRead(grp)
        await toggleVia(grp)
        await expect(grp).toHaveAttribute('aria-expanded', 'true')
        table['board-group-summary'].hover = await hoverRead(grp)
      }

      table['board-epic-summary'] = {}
      {
        const card = board.epicCard(epicNumber)
        const sum = card.locator('.board-epic-summary')
        await expect(sum).toBeVisible()
        if (await isOpen(card)) await toggleVia(sum)
        table['board-epic-summary'].collapsed = await restRead(sum)
        await toggleVia(sum)
        await expect(card).toHaveJSProperty('open', true)
        table['board-epic-summary'].open = await restRead(sum)
        table['board-epic-summary'].hover = await hoverRead(sum)
      }

      table['board-iteration-summary'] = {}
      table['board-iteration-summary-current'] = {}
      {
        const card = board.iterationCard(iterationId)
        await expect(card).toBeVisible()
        const sum = board.iterationSummary(iterationId)
        await expect(card).toHaveAttribute('data-current-iteration', 'true')
        if (!(await isOpen(card))) await toggleVia(sum)
        table['board-iteration-summary-current'].open = await restRead(sum)
        await toggleVia(sum)
        await expect(card).toHaveJSProperty('open', false)
        table['board-iteration-summary'].collapsed = await restRead(sum)
        await toggleVia(sum)
        await expect(card).toHaveJSProperty('open', true)
        table['board-iteration-summary'].open = await restRead(sum)
        table['board-iteration-summary'].hover = await hoverRead(sum)
        table['board-iteration-summary-current'].hover = await hoverRead(sum)
      }

      table['board-iterations-section-summary'] = {}
      {
        const sec = board.activePlannedSection()
        const sum = board.activePlannedSummary()
        await expect(sum).toBeVisible()
        if (!(await isOpen(sec))) await toggleVia(sum)
        table['board-iterations-section-summary'].open = await restRead(sum)
        await toggleVia(sum)
        await expect(sec).toHaveJSProperty('open', false)
        table['board-iterations-section-summary'].collapsed = await restRead(sum)
        await toggleVia(sum)
        await expect(sec).toHaveJSProperty('open', true)
        table['board-iterations-section-summary'].hover = await hoverRead(sum)
      }

      const shotPath = join(tmpdir(), 'bpmux-1519-dark-board.png')
      await win.screenshot({ path: shotPath, fullPage: false })

      expect(tokens.base, 'dark --bg-base must not resolve to light white').not.toBe(
        'rgb(255, 255, 255)'
      )

      const restingTypes = [
        'board-group-summary',
        'board-epic-summary',
        'board-iteration-summary',
        'board-iterations-section-summary'
      ]
      for (const k of restingTypes) {
        expect(table[k].collapsed, `${k} resting background must be --bg-base`).toBe(tokens.base)
        expect(table[k].open, `${k} open background must be --bg-base`).toBe(tokens.base)
        expect(
          table[k].hover,
          `${k} hover background must be --bg-highlighted (uniform toggle-header affordance across all disclosure-summaries)`
        ).toBe(tokens.highlighted)
      }
      expect(
        table['board-iteration-summary-current'].open,
        'current-iteration summary open background must be --bg-base (highlight adds border-left + bold title, not a bg change)'
      ).toBe(tokens.base)
      expect(
        table['board-iteration-summary-current'].hover,
        'current-iteration summary hover background must be --bg-highlighted'
      ).toBe(tokens.highlighted)
    } finally {
      await app.close()
      await mockGh.close()
    }
  })
})
