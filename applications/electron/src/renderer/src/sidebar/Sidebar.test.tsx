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

describe('#1605 Sidebar — coordination overview: addressed who↔whom edges discriminate active from idle', () => {
  const at = (msAgo: number): string => new Date(Date.now() - msAgo).toISOString()
  const chat = (over: Partial<ChatMessage>): ChatMessage => ({
    id: 'c',
    from: 'dev:claude-code',
    text: 'coordinate',
    ts: at(30_000),
    to: null,
    addressee: null,
    messageType: 'chat',
    ...over
  })
  const render = (messages: ChatMessage[]): string =>
    renderToStaticMarkup(
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

  test('a recently @-addressed pair is ACTIVE (recency-dot) while an un-coordinated pair is IDLE (dimmed) — discrimination, not pair-existence', () => {
    const html = render([chat({ id: '1', addressee: 'pm', ts: at(30_000) })])
    expect(html).toMatch(/data-test="sidebar-group-dev\+pm"[^>]*data-coordination="active"/)
    expect(html).toContain('data-test="coordination-dot-dev+pm"')
    expect(html).toMatch(/data-test="sidebar-group-designer\+pm"[^>]*data-coordination="idle"/)
    expect(html).not.toContain('data-test="coordination-dot-designer+pm"')
    expect(html).toContain('sidebar-item--idle')
  })

  test('the active pair sorts ABOVE the idle pairs (active-now rises to the top of GROUPS)', () => {
    const html = render([chat({ id: '1', addressee: 'pm', ts: at(30_000) })])
    expect(html.indexOf('sidebar-group-dev+pm')).toBeLessThan(
      html.indexOf('sidebar-group-designer+pm')
    )
  })

  test('two active pairs sort most-recent-first', () => {
    const html = render([
      chat({ id: '1', from: 'dev:claude-code', addressee: 'pm', ts: at(4 * 60_000) }),
      chat({ id: '2', from: 'designer:claude-code', addressee: 'dev', ts: at(20_000) })
    ])
    expect(html.indexOf('sidebar-group-designer+dev')).toBeLessThan(
      html.indexOf('sidebar-group-dev+pm')
    )
  })

  test('an un-addressed broadcast does NOT activate any pair (the broadcast noise is correctly no-edge)', () => {
    const html = render([chat({ id: '1', addressee: null, to: null, ts: at(10_000) })])
    expect(html).not.toMatch(/data-coordination="active"/)
    expect(html).not.toContain('data-test="coordination-dot-dev+pm"')
  })
})
