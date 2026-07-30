import { describe, expect, test } from 'vitest'
import type { StateDeltaPayload } from '@transport/types'
import { mirrorDeltaReceived } from '@state/actions'
import { boardFold, initialBoard } from '../board/fold'
import { initialMirror, mirrorFold } from '../mirror/fold'

type Owner =
  | 'board'
  | 'mirror'
  | 'chat — decode routes history to historyLoaded before the store'
  | 'agent event — the renderer deliberately ignores it'

const ledger: Record<StateDeltaPayload['domain'], Owner> = {
  reactions: 'mirror',
  reaction_updated: 'mirror',
  roster: 'mirror',
  roster_member_upserted: 'mirror',
  roster_member_removed: 'mirror',
  ci: 'mirror',
  ci_poll: 'mirror',
  sync_poll: 'mirror',
  stories: 'board',
  story_upserted: 'board',
  story_removed: 'board',
  story_order: 'board',
  epics: 'board',
  epic_upserted: 'board',
  epic_removed: 'board',
  epic_order: 'board',
  outcomes: 'board',
  outcome_upserted: 'board',
  outcome_removed: 'board',
  outcome_order: 'board',
  iterations: 'board',
  iteration_upserted: 'board',
  iteration_removed: 'board',
  iteration_order: 'board',
  roadmap: 'board',
  roadmap_card_upserted: 'board',
  roadmap_card_removed: 'board',
  roadmap_order: 'board',
  board_invalidated: 'board',
  history: 'chat — decode routes history to historyLoaded before the store',
  iteration_close: 'agent event — the renderer deliberately ignores it',
  iteration_close_blocked: 'agent event — the renderer deliberately ignores it'
}

describe('every wire domain has exactly one owner — the ledger is total, so a new domain cannot land unnoticed', () => {
  const boardState = initialBoard
  const mirrorState = { mirror: initialMirror }

  const probe = (delta: StateDeltaPayload): { board: boolean; mirror: boolean } => {
    const action = mirrorDeltaReceived(delta, '2026-07-30T00:00:00Z')
    return {
      board: boardFold(boardState, action) !== boardState,
      mirror: mirrorFold(mirrorState, action) !== mirrorState
    }
  }

  test('a board-owned domain moves the board fold and only the board fold', () => {
    expect(ledger.story_order).toEqual('board')
    expect(probe({ domain: 'story_order', ids: [7] })).toEqual({ board: true, mirror: false })
  })

  test('a mirror-owned domain moves the mirror fold and only the mirror fold', () => {
    expect(ledger.roster_member_removed).toEqual('mirror')
    expect(probe({ domain: 'roster_member_removed', routing_id: 'dev-a' })).toEqual({
      board: false,
      mirror: true
    })
  })

  test('an agent-facing domain moves nothing — the renderer ignores it on purpose', () => {
    expect(ledger.iteration_close).toContain('ignores')
    expect(probe({ domain: 'iteration_close', promoted: [], deleted_marker_id: null })).toEqual({
      board: false,
      mirror: false
    })
  })
})
