import { BoardPage } from './pages/boardPage'
import { launchTwoElectronsSharedBackend } from './helpers/launch'
import { test, expect } from '@playwright/test'

test.describe('two instances stay in sync over the WS — every board mutation made in one is live in the other', () => {
  test('seed_stories iteration in instance A propagates to instance B via WS state-delta + BOARD_INVALIDATE', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)

      await expect(board1.iterationGrip(9501)).toHaveCount(0)
      await expect(board2.iterationGrip(9501)).toHaveCount(0)

      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9501,
              title: 'Iteration Pavo',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              id: 9502,
              title: 'pavo-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Pavo.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9501)).toBeVisible({ timeout: 15_000 })
      await expect(board2.iterationGrip(9501)).toBeVisible({ timeout: 15_000 })

      await expect(
        board1.iterationRow(9501).getByText('Iteration Pavo', { exact: true })
      ).toBeVisible()
      await expect(
        board2.iterationRow(9501).getByText('Iteration Pavo', { exact: true })
      ).toBeVisible()
    } finally {
      await cleanup()
    }
  })

  test('reorder in instance A propagates to instance B via WS state-delta', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9301,
              title: 'Iteration Reticulum',
              story_type: 'iteration',
              initial_state: 'icebox',
              body: '<!-- bpmux-relationships: {"stories":[9311,9312,9313]} -->'
            },
            { id: 9311, title: 'reticulum-alpha', story_type: 'feature', initial_state: 'icebox' },
            { id: 9312, title: 'reticulum-bravo', story_type: 'feature', initial_state: 'icebox' },
            { id: 9313, title: 'reticulum-charlie', story_type: 'feature', initial_state: 'icebox' }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await win1.reload()
      await win2.reload()
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9301)).toBeVisible({ timeout: 20_000 })
      await expect(board2.iterationGrip(9301)).toBeVisible({ timeout: 20_000 })

      const initial1 = await board1.memberOrder(9301)
      const initial2 = await board2.memberOrder(9301)
      expect(initial1).toEqual([9311, 9312, 9313])
      expect(initial2).toEqual([9311, 9312, 9313])

      const reorderRes = await fetch(`${apiBase}/api/rpc/add_story_to_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
          'X-BPMUX-Agent': 'pm'
        },
        body: JSON.stringify({ iteration_id: 9301, story_id: 9313, position: 1 })
      })
      expect(reorderRes.ok, await reorderRes.text()).toBe(true)

      await expect
        .poll(() => board1.memberOrder(9301), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
      await expect
        .poll(() => board2.memberOrder(9301), { timeout: 15_000, intervals: [200] })
        .toEqual([9313, 9311, 9312])
    } finally {
      await cleanup()
    }
  })

  test('move a member from iteration A to iteration B in one instance propagates to the other', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9601,
              title: 'Iteration Volans',
              story_type: 'iteration',
              initial_state: 'icebox',
              body: '<!-- bpmux-relationships: {"stories":[9611,9620]} -->'
            },
            {
              id: 9602,
              title: 'Iteration Crater',
              story_type: 'iteration',
              initial_state: 'icebox',
              body: '<!-- bpmux-relationships: {"stories":[9621]} -->'
            },
            { id: 9611, title: 'volans-member', story_type: 'feature', initial_state: 'icebox' },
            { id: 9620, title: 'volans-anchor', story_type: 'feature', initial_state: 'icebox' },
            { id: 9621, title: 'crater-anchor', story_type: 'feature', initial_state: 'icebox' }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9601)).toBeVisible({ timeout: 15_000 })
      await expect(board2.iterationGrip(9602)).toBeVisible({ timeout: 15_000 })

      const memberIn = async (board: BoardPage, iteration: number): Promise<boolean> =>
        (await board.memberOrder(iteration)).includes(9611)

      expect(await memberIn(board1, 9601)).toBe(true)
      expect(await memberIn(board1, 9602)).toBe(false)
      expect(await memberIn(board2, 9601)).toBe(true)
      expect(await memberIn(board2, 9602)).toBe(false)

      const moveRes = await fetch(`${apiBase}/api/rpc/add_story_to_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
          'X-BPMUX-Agent': 'pm'
        },
        body: JSON.stringify({ iteration_id: 9602, story_id: 9611, position: 1 })
      })
      expect(moveRes.ok, await moveRes.text()).toBe(true)

      await expect
        .poll(() => memberIn(board1, 9602), { timeout: 15_000, intervals: [200] })
        .toBe(true)
      await expect
        .poll(() => memberIn(board2, 9602), { timeout: 15_000, intervals: [200] })
        .toBe(true)

      expect(await memberIn(board1, 9601)).toBe(false)
      expect(await memberIn(board2, 9601)).toBe(false)
    } finally {
      await cleanup()
    }
  })

  test('iteration-order label mutation in instance A propagates container order in instance B', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9701,
              title: 'Iteration Norma',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.100']
            },
            {
              id: 9702,
              title: 'Iteration Octans',
              story_type: 'iteration',
              initial_state: 'icebox',
              labels: ['iteration-order.200']
            },
            {
              id: 9710,
              title: 'norma-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Norma.100']
            },
            {
              id: 9711,
              title: 'octans-member',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Octans.100']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect
        .poll(() => board1.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9701, 9702])
      await expect
        .poll(() => board2.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9701, 9702])

      const mutateRes = await fetch(`${apiBase}/api/stories/9702/labels`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          add: ['iteration-order.50'],
          remove: ['iteration-order.200']
        })
      })
      expect(mutateRes.ok).toBe(true)

      await expect
        .poll(() => board1.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
      await expect
        .poll(() => board2.iterationOrder(), { timeout: 15_000, intervals: [200] })
        .toEqual([9702, 9701])
    } finally {
      await cleanup()
    }
  })

  test('close_iteration in instance A closes the container in instance B via WS state-delta; members are NOT promoted (promote-on-close retired #1556)', async () => {
    const { win1, win2, apiBase, authToken, cleanup } = await launchTwoElectronsSharedBackend()
    try {
      const seedRes = await fetch(`${apiBase}/api/test/seed_stories`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({
          stories: [
            {
              id: 9401,
              title: 'Iteration Cetus',
              story_type: 'iteration',
              initial_state: 'icebox'
            },
            {
              id: 9411,
              title: 'cetus-alpha',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Cetus.100']
            },
            {
              id: 9412,
              title: 'cetus-bravo',
              story_type: 'feature',
              initial_state: 'icebox',
              labels: ['iteration-Cetus.200']
            }
          ]
        })
      })
      expect(seedRes.ok).toBe(true)

      await win1.reload()
      await win2.reload()
      const board1 = new BoardPage(win1)
      const board2 = new BoardPage(win2)
      await board1.openTab('Plan')
      await board2.openTab('Plan')

      await expect(board1.iterationGrip(9401)).toBeVisible({ timeout: 20_000 })
      await expect(board2.iterationGrip(9401)).toBeVisible({ timeout: 20_000 })

      const stateOf = async (): Promise<{
        iterationClosed: boolean
        memberStates: Record<number, string>
      }> => {
        const body = await fetch(`${apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${authToken}` }
        }).then(
          (r) =>
            r.json() as Promise<{
              stories: { id: number; state: string }[]
            }>
        )
        const iterations = await fetch(`${apiBase}/api/iterations`, {
          headers: { Authorization: `Bearer ${authToken}` }
        }).then(
          (r) =>
            r.json() as Promise<{
              iterations: { id: number; complete: boolean }[]
            }>
        )
        return {
          iterationClosed: iterations.iterations.find((s) => s.id === 9401)?.complete === true,
          memberStates: Object.fromEntries(
            body.stories.filter((s) => s.id === 9411 || s.id === 9412).map((s) => [s.id, s.state])
          )
        }
      }

      const closeRes = await fetch(`${apiBase}/api/close_iteration`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`
        },
        body: JSON.stringify({ iteration_id: 9401 })
      })
      expect(closeRes.ok).toBe(true)

      await board1.openClosedIterationsSection()
      await board2.openClosedIterationsSection()
      await expect(
        board1.closedIterationsSection().getByText('Iteration Cetus', { exact: true })
      ).toBeVisible({ timeout: 15_000 })
      await expect(
        board2.closedIterationsSection().getByText('Iteration Cetus', { exact: true })
      ).toBeVisible({ timeout: 15_000 })

      const finalState = await stateOf()
      expect(finalState.iterationClosed).toBe(true)
      expect(finalState.memberStates[9411]).toBe('icebox')
      expect(finalState.memberStates[9412]).toBe('icebox')
    } finally {
      await cleanup()
    }
  })
})
