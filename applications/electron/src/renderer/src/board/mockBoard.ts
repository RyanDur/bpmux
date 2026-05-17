import type { BoardResponse } from './types'

export const MOCK_BOARD_RESPONSE: BoardResponse = {
  fetched_at: new Date().toISOString(),
  stories: [
    {
      number: 324,
      title: 'Side-panel board surface — story list + drill-in',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-18T12:00:00Z',
      body: '## What\nRight-docked board that lists current stories, groups by state, and drills into a single story on click.\n\n## Why\nRyan needs the board visible alongside chat during sessions so story state is continuously present, not a tool-call away.\n\n## Acceptance criteria\n- Groups collapse / expand\n- Arrow nav works\n- Drill-in shows body, accept/reject visible for delivered'
    },
    {
      number: 325,
      title: 'Project panel drill-in: metadata + actions',
      state: 'in-progress',
      labels: ['Feature', 'in-progress'],
      assignees: ['dev'],
      created_at: '2026-04-19T08:30:00Z',
      body: 'Drill-in shows state, labels, assignees, body — plus Accept/Reject for the owner on delivered stories.'
    },
    {
      number: 326,
      title: 'Accept button fires from keyboard enter',
      state: 'delivered',
      labels: ['Bug'],
      assignees: [],
      created_at: '2026-04-20T10:15:00Z',
      body: 'When a delivered story is open + Accept has focus, Enter should fire the accept action. Currently only mouse click works.'
    }
  ]
}
