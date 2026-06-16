import { afterEach, describe, expect, test, vi } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { cleanup, fireEvent, render } from '@testing-library/react'
import Sidebar from './Sidebar'
import type { MessageFilter } from '../hooks/useMessageFilter'
import type { ChatMessage, RosterMemberWire } from '../transport/types'

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
  wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
  wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' }),
  wire({ name: 'pm', identity: 'pm:claude-code', role: 'Pm' }),
  wire({ name: 'designer', identity: 'designer:claude-code', role: 'Designer' })
]

const RYAN_ONLY_MESSAGES: ChatMessage[] = [
  {
    id: 'msg-1',
    from: 'ryan:electron',
    text: 'i am here',
    ts: '2026-04-23T12:00:00Z',
    to: null,
    addressee: null,
    messageType: 'chat'
  }
]

afterEach(cleanup)

describe('Sidebar — slice 5g+5g-fix mirror-roster derivation (replaces desk-check-402.spec.ts)', () => {
  test('non-speaking peer roles from mirror.roster render as direct entries', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).toContain('data-test="sidebar-direct-dev"')
    expect(html).toContain('data-test="sidebar-direct-pm"')
    expect(html).toContain('data-test="sidebar-direct-designer"')
  })

  test('viewer (ryan) excluded from sidebar-direct entries (#393 AC1 self-filter)', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toContain('data-test="sidebar-direct-ryan"')
  })

  test('#1246: only-self in roster shows NO peer direct-entries (no message-history fallback)', () => {
    const SELF_ONLY: RosterMemberWire[] = [
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' })
    ]
    const messages: ChatMessage[] = [
      {
        id: 'm1',
        from: 'ryan:electron',
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
        mirrorRoster={SELF_ONLY}
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
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).toContain('data-test="sidebar-group-designer+dev"')
    expect(html).toContain('data-test="sidebar-group-designer+pm"')
    expect(html).toContain('data-test="sidebar-group-dev+pm"')
    expect(html).not.toContain('data-test="sidebar-group-designer+dev+pm"')
  })
})

describe('Sidebar — targeting survives a GROUPS surface-change (#1605 owner-walk regression)', () => {
  test('clicking a group scopes the recipient to that group', () => {
    const onSelectRecipient = vi.fn()
    const { getByTestId } = render(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    fireEvent.click(getByTestId('sidebar-group-designer+dev'))
    expect(onSelectRecipient).toHaveBeenCalledTimes(1)
    expect([...onSelectRecipient.mock.calls[0][0]].sort()).toEqual(['designer', 'dev'])
  })

  test('#1733: a 2-peer roster scopes its only group to that exact pair, not broadcast', () => {
    const onSelectRecipient = vi.fn()
    const TWO_PEER_ROSTER: RosterMemberWire[] = [
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' }),
      wire({ name: 'pm', identity: 'pm:claude-code', role: 'Pm' })
    ]
    const { getByTestId } = render(
      <Sidebar
        recipient={null}
        allView
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={TWO_PEER_ROSTER}
      />
    )
    fireEvent.click(getByTestId('sidebar-group-dev+pm'))
    expect(onSelectRecipient).toHaveBeenCalledTimes(1)
    expect(onSelectRecipient.mock.calls[0][0]).not.toBeNull()
    expect([...onSelectRecipient.mock.calls[0][0]].sort()).toEqual(['dev', 'pm'])
  })

  test('clicking a direct role scopes the recipient to that role', () => {
    const onSelectRecipient = vi.fn()
    const { getByTestId } = render(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={onSelectRecipient}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    fireEvent.click(getByTestId('sidebar-direct-dev'))
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
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev' })
    ]
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
        mirrorRoster={ROSTER_HAS_DEV_ONLY}
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

describe('#1457 Sidebar — Pictor-rework: retire +ryan / 3-role-group / BPMUX section', () => {
  test('Scenario 1: GROUP rows do NOT carry +ryan affordance (owner-is-constant; canon-cite 18:31 UTC "I am already here")', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toContain('+ryan')
    expect(html).not.toContain('sidebar-item-owner-affordance')
    expect(html).not.toContain('owner ryan also observes')
    expect(html).not.toMatch(/data-test="sidebar-group-[^"]+-owner"/)
  })

  test('Scenario 2: BPMUX section retired entirely (filter pills handle bpmux class)', () => {
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
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toContain('<h3 class="sidebar-heading">bpmux</h3>')
    expect(html).not.toContain('<h3 class="sidebar-heading">system</h3>')
    expect(html).not.toContain('data-test="sidebar-bpmux"')
    expect(html).not.toContain('data-test="sidebar-bpmux-empty"')
    expect(html).not.toContain('data-test="sidebar-system"')
  })

  test('Scenario 3: 3-role all-members group retired (subsumed by "all conversations")', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toMatch(/data-test="sidebar-group-designer\+dev\+pm"/)
    expect(html).not.toContain('data-all-members="true"')
  })

  test('Scenario 4: 2-role GROUP rooms preserved with member-list-naming', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toContain('>broadcast<')
    expect(html).toMatch(/data-test="sidebar-group-designer\+dev"/)
    expect(html).toMatch(/data-test="sidebar-group-designer\+pm"/)
    expect(html).toMatch(/data-test="sidebar-group-dev\+pm"/)
  })

  test('Scenario 5: "all conversations" entry preserved with explanatory title', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).toContain('data-test="sidebar-all"')
    expect(html).toContain('all conversations')
    expect(html).toMatch(/title="meta-view of every channel"/)
  })

  test('Scenario 6 canon: zero <div> + zero <span> in rendered sidebar (feedback_html5_semantic_no_divs_spans)', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toMatch(/<div\b/)
    expect(html).not.toMatch(/<span\b/)
  })

  test('Scenario 7: presence-dot still renders per DIRECT role (semantic refactored to <i>)', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-designer"/)
    expect(html).toMatch(/data-test="presence-dot-dev"/)
    expect(html).toMatch(/data-test="presence-dot-pm"/)
  })
})

describe('Sidebar — presence dot honestly reflects wire roster state, never green from mere connection (#1719)', () => {
  test('wire presence Online (connected, no signal yet) shows UNKNOWN hollow, NEVER available/green (#1739 S1: the dot reflects a pushed state; mere connection is unknown)', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="unknown"/)
    expect(html).not.toMatch(/data-test="presence-dot-dev"[^>]*data-presence="available"/)
  })

  test('wire presence Blocked (owner-parked) shows a red blocked dot and the blocked label', () => {
    const BLOCKED_ROSTER: RosterMemberWire[] = [
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Blocked' })
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={BLOCKED_ROSTER}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="blocked"/)
    expect(html).toMatch(/data-test="presence-label-dev"[^>]*>blocked</)
  })

  test('a roster member with no wire presence entry shows unknown (neutral), never available/green', () => {
    const NO_PRESENCE: RosterMemberWire[] = [
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
      { name: 'dev', identity: 'dev:claude-code', role: 'Dev' } as RosterMemberWire
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={NO_PRESENCE}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="unknown"/)
    expect(html).not.toMatch(/data-test="presence-dot-dev"[^>]*data-presence="available"/)
  })

  test('no path yields data-presence="available" from connection alone in S1', () => {
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={ROSTER_FIXTURE}
      />
    )
    expect(html).not.toMatch(/data-presence="available"/)
  })

  test('wire presence Busy maps to the busy dot', () => {
    const BUSY_ROSTER: RosterMemberWire[] = [
      wire({ name: 'ryan', identity: 'ryan:electron', role: 'Owner' }),
      wire({ name: 'dev', identity: 'dev:claude-code', role: 'Dev', presence: 'Busy' })
    ]
    const html = renderToStaticMarkup(
      <Sidebar
        recipient={null}
        allView={false}
        onSelectRecipient={() => {}}
        onSelectAllView={() => {}}
        messages={RYAN_ONLY_MESSAGES}
        filter={filter}
        mirrorRoster={BUSY_ROSTER}
      />
    )
    expect(html).toMatch(/data-test="presence-dot-dev"[^>]*data-presence="busy"/)
  })
})
