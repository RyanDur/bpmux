// #1302 rework — RTL+real-backend live-walk gate for renderer
// iteration-grouping. Sister to #1304 archetype-pattern: seed
// iteration-issue + N member-stories via test-seed-API, mount via
// renderApp, assert grouped-visibility in DOM. The failing-test pins
// the user-visible behavior PM + designer + dev each must live-fire-
// verify before pass_desk_check (per
// feedback_does_it_work_team_uniform_gate +
// feedback_pre_owner_walk_validation).

import { afterEach, describe, expect, inject, it } from 'vitest'

import { renderApp, type RenderAppResult } from '../../../../test/fixtures/renderApp'
import { seedStories } from '../../../../test/fixtures/seedMessages'

describe('#1302 rework — iteration-grouping in icebox', () => {
  let rendered: RenderAppResult | null = null

  afterEach(() => {
    rendered?.cleanup()
    rendered = null
  })

  it(
    'iteration-issue + member-stories render visually-grouped (sister to Epic+children)',
    async () => {
      const port = inject('bpmuxBackendPort')
      const authToken = inject('bpmuxAuthToken')
      const apiBase = `http://127.0.0.1:${port}`

      // Seed an Iteration issue + 3 member-stories with matching
      // iteration_id. The canonical-via-GH substrate per Epic #1189:
      // story_type=Iteration + per-member iteration-#NNNN label
      // decoded into iteration_id.
      await seedStories(apiBase, authToken, [
        {
          number: 9001,
          title: 'Iteration Cassiopeia',
          story_type: 'iteration',
          initial_state: 'icebox'
        },
        {
          number: 9101,
          title: 'member story 9101',
          story_type: 'feature',
          initial_state: 'icebox',
          iteration_id: 9001
        },
        {
          number: 9102,
          title: 'member story 9102',
          story_type: 'feature',
          initial_state: 'icebox',
          iteration_id: 9001
        },
        {
          number: 9103,
          title: 'member story 9103',
          story_type: 'feature',
          initial_state: 'icebox',
          iteration_id: 9001
        }
      ])

      rendered = renderApp()

      // (a) Iteration section exists + names the iteration-issue
      const section = await rendered.findByTestId('board-iterations', undefined, {
        timeout: 15_000
      })
      expect(section).toBeTruthy()

      // (b) Iteration-issue rendered as parent-row with its title
      const iterationTitle = await rendered.findByTestId(
        'board-iteration-9001-title',
        undefined,
        { timeout: 15_000 }
      )
      expect(iterationTitle.textContent).toBe('Iteration Cassiopeia')

      // (c) Member-count visible — sister-affordance to Epic progress
      const memberCount = rendered.getByTestId('board-iteration-9001-count')
      expect(memberCount.textContent).toContain('3')

      // (d) Member-stories visually grouped under iteration via
      // MiniBoard — sister-pattern to Epic+children grouping
      const miniBoard = rendered.getByTestId('board-epic-9001-miniboard')
      expect(miniBoard).toBeTruthy()
      expect(miniBoard.textContent).toContain('member story 9101')
      expect(miniBoard.textContent).toContain('member story 9102')
      expect(miniBoard.textContent).toContain('member story 9103')

      // (e) Member-stories NOT double-rendered as flat icebox rows
      // (the flat-state column excludes stories whose iteration_id
      // matches an active iteration-issue). Sister to Epic+children
      // pattern where children DO still appear in flat columns —
      // iterations DIFFER here intentionally so the user-visible
      // grouping is unambiguous.
      const iceboxRows = rendered.container.querySelectorAll(
        '#board-group-icebox-content [data-test^="board-story-91"]'
      )
      expect(iceboxRows.length).toBe(0)
    },
    25_000
  )

  it('add_stories_to_iteration rejects self-reference at substrate (sister-Bug #1336)', async () => {
    const port = inject('bpmuxBackendPort')
    const authToken = inject('bpmuxAuthToken')
    const apiBase = `http://127.0.0.1:${port}`

    // Seed the iteration-issue so it exists; then attempt to add it
    // as a member of itself via the canonical REST endpoint. The
    // validation gate must reject with 400 + a named error.
    await seedStories(apiBase, authToken, [
      {
        number: 9200,
        title: 'Iteration Self-Reference Bait',
        story_type: 'iteration',
        initial_state: 'icebox'
      }
    ])

    const res = await fetch(`${apiBase}/api/add_stories_to_iteration`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${authToken}`
      },
      body: JSON.stringify({
        iteration_number: 9200,
        story_ids: [9200]
      })
    })
    expect(res.status).toBe(400)
    const body = await res.text()
    expect(body).toContain('cannot be a member of itself')
  })
})
