import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Reactions, { EMOJI_PALETTE, ReactionAddButton } from './Reactions'
import type { AggregatedReaction } from '../hooks/useReactions'

const SEEDED_REACTIONS: AggregatedReaction[] = [
  {
    message_key: '2026-04-22T16:01:00.000Z|dev',
    emoji: '👍',
    count: 2,
    users: ['ryan', 'pm']
  },
  {
    message_key: '2026-04-22T16:01:00.000Z|dev',
    emoji: '🎉',
    count: 1,
    users: ['designer']
  }
]

describe('Reactions — direct prop injection (replaces desk-check-340.spec.ts)', () => {
  test('seeded reactions render as pills with emoji, count, and aria-label', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={SEEDED_REACTIONS} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toContain('data-test="reaction-pill-👍"')
    expect(html).toContain('data-test="reaction-pill-🎉"')
    expect(html).toContain('aria-label="👍 2 reactions from ryan, pm"')
    expect(html).toContain('aria-label="🎉 1 reactions from designer"')
    expect(html).toContain('<small>2</small>')
    expect(html).toContain('<small>1</small>')
  })

  test('pill aria-pressed reflects whether viewer is in users list', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={SEEDED_REACTIONS} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toMatch(/data-test="reaction-pill-👍"[^>]*aria-pressed="true"/)
    expect(html).toMatch(/data-test="reaction-pill-🎉"[^>]*aria-pressed="false"/)
  })

  test('reacted pill receives reaction-pill--reacted class', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={SEEDED_REACTIONS} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toContain('reaction-pill reaction-pill--reacted')
  })

  test('add-reaction button renders alongside pills when reactions present', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={SEEDED_REACTIONS} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toContain('data-test="reaction-add"')
    expect(html).toContain('aria-label="Add reaction"')
    expect(html).toContain('aria-haspopup="menu"')
    expect(html).toContain('aria-expanded="false"')
  })

  test('returns null when reactions are undefined and picker not open', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={undefined} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toBe('')
  })

  test('returns null when reactions are empty and picker not open', () => {
    const html = renderToStaticMarkup(
      <Reactions reactions={[]} viewer="ryan" onToggle={() => {}} />
    )
    expect(html).toBe('')
  })
})

describe('ReactionAddButton — first-reaction affordance for unreacted messages', () => {
  test('renders with aria-label="Add reaction" and reaction-add-first data-test', () => {
    const html = renderToStaticMarkup(<ReactionAddButton onToggle={() => {}} />)
    expect(html).toContain('data-test="reaction-add-first"')
    expect(html).toContain('aria-label="Add reaction"')
    expect(html).toContain('aria-haspopup="menu"')
    expect(html).toContain('aria-expanded="false"')
  })
})

describe('EMOJI_PALETTE — picker palette identity', () => {
  test('palette contains the expected 6 emoji in order', () => {
    expect(EMOJI_PALETTE).toEqual(['👍', '🎉', '❤️', '🚀', '👀', '❌'])
  })
})
