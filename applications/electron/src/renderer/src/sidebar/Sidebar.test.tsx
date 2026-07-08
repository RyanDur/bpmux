import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { act, cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import Sidebar from './Sidebar'
import type { MessageFilter } from '../hooks/useMessageFilter'
import type { ChatMessage, RosterMemberWire } from '../transport/types'
import { dispatchState, resetStateForTest } from '../transport/state/store'
import { mirrorDeltaReceived } from '../transport/state/actions'

const filter: MessageFilter = {
  visible: new Set(['chat']),
  toggle: () => {},
  reset: () => {},
  setOnly: () => {}
}

const wire = (overrides: Partial<RosterMemberWire>): RosterMemberWire => ({
  routing_id: 'sess-dev',
  name: 'dev',
  identity: 'dev:claude-code',
  role: 'Dev',
  presence: 'Online',
  ...overrides
})

const ROSTER_FIXTURE: RosterMemberWire[] = [
  wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
  wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' }),
  wire({ name: 'pm', identity: 'pm:claude-code', role: 'Pm' }),
  wire({ name: 'designer', identity: 'designer:claude-code', role: 'Designer' })
]

const pushRoster = (members: RosterMemberWire[]): void =>
  dispatchState(mirrorDeltaReceived({ domain: 'roster', members }, '2026-01-01T00:00:00Z'))

const OWNER_ONLY_MESSAGES: ChatMessage[] = [
  {
    id: 'msg-1',
    from: 'owner:electron',
    text: 'i am here',
    ts: '2026-04-23T12:00:00Z',
    to: null,
    addressee: null,
    messageType: 'chat'
  }
]

afterEach(() => {
  cleanup()
  resetStateForTest()
})

describe('Sidebar — slice 5g+5g-fix mirror-roster derivation (replaces desk-check-402.spec.ts)', () => {
  test('non-speaking peer roles from mirror.roster render as direct entries', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toContain('data-test="sidebar-direct-dev"')
    expect(html).toContain('data-test="sidebar-direct-pm"')
    expect(html).toContain('data-test="sidebar-direct-designer"')
  })

  test('viewer (owner) excluded from sidebar-direct entries (#393 AC1 self-filter)', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toContain('data-test="sidebar-direct-owner"')
  })

  test('#1246: only-self in roster shows NO peer direct-entries (no message-history fallback)', () => {
    const SELF_ONLY: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' })
    ]
    pushRoster(SELF_ONLY)
    const messages: ChatMessage[] = [
      {
        id: 'm1',
        from: 'owner:electron',
        text: 'addressed',
        ts: '2026-04-23T12:00:00Z',
        to: ['dev'],
        addressee: 'dev',
        messageType: 'chat'
      }
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={messages}
        filter={filter}
      />
    )
    expect(html).not.toContain('data-test="sidebar-direct-dev"')
  })

  test('#1457: broadcast button retired — all conversations entry remains', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={[]}
        filter={filter}
      />
    )
    expect(html).not.toContain('data-test="sidebar-broadcast"')
    expect(html).toContain('data-test="sidebar-all"')
  })

  test('combinatorial groups derive from peer roster (excludes self)', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toContain('data-test="sidebar-group-designer+dev"')
    expect(html).toContain('data-test="sidebar-group-designer+pm"')
    expect(html).toContain('data-test="sidebar-group-dev+pm"')
    expect(html).not.toContain('data-test="sidebar-group-designer+dev+pm"')
  })
})

describe('Sidebar — targeting survives a GROUPS surface-change (#1605 owner-walk regression)', () => {
  test('clicking a group scopes the recipient to that group', async () => {
    pushRoster(ROSTER_FIXTURE)
    const onSelectRecipient = vi.fn()
    render(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    await userEvent.click(screen.getByTestId('sidebar-group-designer+dev'))
    expect(onSelectRecipient).toHaveBeenCalledTimes(1)
    expect([...onSelectRecipient.mock.calls[0][0]].sort()).toEqual(['designer', 'dev'])
  })

  test('#1733: a 2-peer roster scopes its only group to that exact pair, not broadcast', async () => {
    const onSelectRecipient = vi.fn()
    const TWO_PEER_ROSTER: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' }),
      wire({ name: 'pm', identity: 'pm:claude-code', role: 'Pm' })
    ]
    pushRoster(TWO_PEER_ROSTER)
    render(
      <Sidebar
        recipient={null}
        allView
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    await userEvent.click(screen.getByTestId('sidebar-group-dev+pm'))
    expect(onSelectRecipient).toHaveBeenCalledTimes(1)
    expect(onSelectRecipient.mock.calls[0][0]).not.toBeNull()
    expect([...onSelectRecipient.mock.calls[0][0]].sort()).toEqual(['dev', 'pm'])
  })

  test('clicking a direct role scopes the recipient to that role', async () => {
    pushRoster(ROSTER_FIXTURE)
    const onSelectRecipient = vi.fn()
    render(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    await userEvent.click(screen.getByTestId('sidebar-direct-dev'))
    expect(onSelectRecipient).toHaveBeenCalledWith(['dev'])
  })
})

describe('#1246 Sidebar — roster is source-of-truth; no message-history fallback', () => {
  test('empty roster + only-message-history renders NO peer direct-entries', () => {
    const messages: ChatMessage[] = [
      {
        id: 'm-dev',
        from: 'dev:claude-code',
        text: 'dev spoke historically',
        ts: '2026-04-23T11:00:10Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      },
      {
        id: 'm-pm',
        from: 'pm:claude-code',
        text: 'pm spoke historically',
        ts: '2026-04-23T11:00:20Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      }
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={messages}
        filter={filter}
      />
    )
    expect(html).not.toContain('data-test="sidebar-direct-dev"')
    expect(html).not.toContain('data-test="sidebar-direct-pm"')
  })

  test('roster present + message-history present → roster wins (peers from roster only)', () => {
    const ROSTER_HAS_DEV_ONLY: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' })
    ]
    pushRoster(ROSTER_HAS_DEV_ONLY)
    const messages: ChatMessage[] = [
      {
        id: 'm-pm',
        from: 'pm:claude-code',
        text: 'pm spoke but is not in roster',
        ts: '2026-04-23T11:00:20Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      }
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={messages}
        filter={filter}
      />
    )
    expect(html).toContain('data-test="sidebar-direct-dev"')
    expect(html).not.toContain('data-test="sidebar-direct-pm"')
  })

  test('combinatorial groups derive from roster only — no roster yields no groups', () => {
    const messages: ChatMessage[] = [
      {
        id: 'm-dev',
        from: 'dev:claude-code',
        text: 'dev',
        ts: '2026-04-23T11:00:10Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      },
      {
        id: 'm-pm',
        from: 'pm:claude-code',
        text: 'pm',
        ts: '2026-04-23T11:00:20Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      }
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={messages}
        filter={filter}
      />
    )
    expect(html).not.toMatch(/data-test="sidebar-group-/)
  })
})

describe('#1457 Sidebar — Pictor-rework: retire +owner / 3-role-group / BPMUX section', () => {
  test('Scenario 1: GROUP rows do NOT carry +owner affordance (owner-is-constant; canon-cite 18:31 UTC "I am already here")', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toContain('+owner')
    expect(html).not.toContain('sidebar-item-owner-affordance')
    expect(html).not.toContain('owner owner also observes')
    expect(html).not.toMatch(/data-test="sidebar-group-[^"]+-owner"/)
  })

  test('Scenario 2: BPMUX section retired entirely (filter pills handle bpmux class)', () => {
    pushRoster(ROSTER_FIXTURE)
    const messages: ChatMessage[] = [
      {
        id: 'sys-1',
        from: 'bpmux',
        text: 'a bpmux broadcast',
        ts: '2026-04-23T12:00:00Z',
        to: null,
        addressee: null,
        messageType: 'system'
      }
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={messages}
        filter={filter}
      />
    )
    expect(html).not.toContain('<h3 class="sidebar-heading">bpmux</h3>')
    expect(html).not.toContain('<h3 class="sidebar-heading">system</h3>')
    expect(html).not.toContain('data-test="sidebar-bpmux"')
    expect(html).not.toContain('data-test="sidebar-bpmux-empty"')
    expect(html).not.toContain('data-test="sidebar-system"')
  })

  test('Scenario 3: 3-role all-members group retired (subsumed by "all conversations")', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toMatch(/data-test="sidebar-group-designer\+dev\+pm"/)
    expect(html).not.toContain('data-all-members="true"')
  })

  test('Scenario 4: 2-role GROUP rooms preserved with member-list-naming', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toContain('>broadcast<')
    expect(html).toMatch(/data-test="sidebar-group-designer\+dev"/)
    expect(html).toMatch(/data-test="sidebar-group-designer\+pm"/)
    expect(html).toMatch(/data-test="sidebar-group-dev\+pm"/)
  })

  test('Scenario 5: "all conversations" entry preserved with explanatory title', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toContain('data-test="sidebar-all"')
    expect(html).toContain('all conversations')
    expect(html).toMatch(/title="meta-view of every channel"/)
  })

  test('Scenario 6 canon: zero <div> + zero <span> in rendered sidebar (feedback_html5_semantic_no_divs_spans)', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toMatch(/<div\b/)
    expect(html).not.toMatch(/<span\b/)
  })

  test('Scenario 7: presence-dot still renders per DIRECT role (semantic refactored to <i>)', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-designer"/)
    expect(html).toMatch(/data-test="presence-dot-dev"/)
    expect(html).toMatch(/data-test="presence-dot-pm"/)
  })
})

describe('Sidebar — presence dot honestly reflects wire roster state, never green from mere connection (#1719)', () => {
  test('wire presence Online (connected, no signal yet) shows UNKNOWN hollow, NEVER available/green (#1739 S1: the dot reflects a pushed state; mere connection is unknown)', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="unknown"/)
    expect(html).not.toMatch(/data-test="presence-dot-dev"[^>]*data-presence="available"/)
  })

  test('wire presence Blocked (owner-parked) shows a red blocked dot and the blocked label', () => {
    const BLOCKED_ROSTER: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Blocked' })
    ]
    pushRoster(BLOCKED_ROSTER)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="blocked"/)
    expect(html).toMatch(/data-test="presence-label-dev"[^>]*>blocked</)
  })

  test('a roster member with no wire presence entry shows unknown (neutral), never available/green', () => {
    const NO_PRESENCE: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
      { name: 'dev', identity: 'dev:claude-code', role: 'Dev' } as RosterMemberWire
    ]
    pushRoster(NO_PRESENCE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="unknown"/)
    expect(html).not.toMatch(/data-test="presence-dot-dev"[^>]*data-presence="available"/)
  })

  test('no path yields data-presence="available" from connection alone in S1', () => {
    pushRoster(ROSTER_FIXTURE)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).not.toMatch(/data-presence="available"/)
  })

  test('wire presence Busy maps to the busy dot', () => {
    const BUSY_ROSTER: RosterMemberWire[] = [
      wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Busy' })
    ]
    pushRoster(BUSY_ROSTER)
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={OWNER_ONLY_MESSAGES}
        filter={filter}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="busy"/)
  })
})

describe('#1749 S3 — terminal=connection: the dot flickers on message-I/O (motion keyed off messages, never colour)', () => {
  const baseProps = {
    recipient: null,
    allView: false,
    onSelectRecipient: () => {},
    onSelectAllView: () => {},
    filter
  }

  test('a directed message flickers BOTH sender and recipient, then clears after the window', () => {
    vi.useFakeTimers()
    try {
      pushRoster(ROSTER_FIXTURE)
      const { rerender } = render(<Sidebar {...baseProps} messages={[]} />)
      expect(screen.getByTestId('presence-dot-dev').className).not.toContain('presence-dot--io')

      const msg: ChatMessage = {
        id: 'io-1',
        from: 'dev:claude-code',
        text: 'on it',
        ts: '2026-06-16T12:00:00Z',
        to: ['pm'],
        addressee: 'pm',
        messageType: 'chat'
      }
      rerender(<Sidebar {...baseProps} messages={[msg]} />)
      expect(screen.getByTestId('presence-dot-dev').className).toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-pm').className).toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-designer').className).not.toContain(
        'presence-dot--io'
      )

      act(() => {
        vi.advanceTimersByTime(2100)
      })
      expect(screen.getByTestId('presence-dot-dev').className).not.toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-pm').className).not.toContain('presence-dot--io')
    } finally {
      vi.useRealTimers()
    }
  })

  test('a broadcast (to=null) flickers every peer dot — it flows to all', () => {
    vi.useFakeTimers()
    try {
      pushRoster(ROSTER_FIXTURE)
      const { rerender } = render(<Sidebar {...baseProps} messages={[]} />)
      const msg: ChatMessage = {
        id: 'b-1',
        from: 'owner:electron',
        text: 'team sync',
        ts: '2026-06-16T12:00:01Z',
        to: null,
        addressee: null,
        messageType: 'chat'
      }
      rerender(<Sidebar {...baseProps} messages={[msg]} />)
      expect(screen.getByTestId('presence-dot-dev').className).toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-pm').className).toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-designer').className).toContain('presence-dot--io')
    } finally {
      vi.useRealTimers()
    }
  })

  test('flicker is orthogonal to colour: a Busy dot that flickers keeps its busy colour', () => {
    vi.useFakeTimers()
    try {
      const BUSY_ROSTER: RosterMemberWire[] = [
        wire({ name: 'owner', identity: 'owner:electron', role: 'Owner' }),
        wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Busy' }),
        wire({ name: 'pm', identity: 'pm:claude-code', role: 'Pm' })
      ]
      pushRoster(BUSY_ROSTER)
      const { rerender } = render(<Sidebar {...baseProps} messages={[]} />)
      const msg: ChatMessage = {
        id: 'io-2',
        from: 'dev:claude-code',
        text: 'shipping',
        ts: '2026-06-16T12:00:02Z',
        to: ['pm'],
        addressee: 'pm',
        messageType: 'chat'
      }
      rerender(<Sidebar {...baseProps} messages={[msg]} />)
      const dot = screen.getByTestId('presence-dot-dev')
      expect(dot.className).toContain('presence-dot--busy')
      expect(dot.className).toContain('presence-dot--io')
      expect(dot.getAttribute('data-presence')).toBe('busy')
    } finally {
      vi.useRealTimers()
    }
  })

  test('message-history at mount does NOT flicker — flicker is a live event, not a replay', () => {
    vi.useFakeTimers()
    try {
      pushRoster(ROSTER_FIXTURE)
      const history: ChatMessage[] = [
        {
          id: 'hist-1',
          from: 'dev:claude-code',
          text: 'earlier',
          ts: '2026-06-16T11:00:00Z',
          to: ['pm'],
          addressee: 'pm',
          messageType: 'chat'
        }
      ]
      render(<Sidebar {...baseProps} messages={history} />)
      expect(screen.getByTestId('presence-dot-dev').className).not.toContain('presence-dot--io')
      expect(screen.getByTestId('presence-dot-pm').className).not.toContain('presence-dot--io')
    } finally {
      vi.useRealTimers()
    }
  })
})
