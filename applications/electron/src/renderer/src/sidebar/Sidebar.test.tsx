import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
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

  test('broadcast entry renders regardless of roster availability', () => {
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
    expect(html).toContain('data-test="sidebar-broadcast"')
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
    expect(html).toContain('data-test="sidebar-group-designer+dev+pm"')
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
