import { afterEach, describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import CiGlyph from './CiGlyph'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'
import type { CiSnapshotWire } from '../transport/types'

const snap = (over: Partial<CiSnapshotWire> & Pick<CiSnapshotWire, 'state'>): CiSnapshotWire => ({
  run_id: 7,
  head_sha: 'abc1234def',
  display_title: 'test commit',
  url: 'https://github.com/acme/bpmux/actions',
  ...over
})

const pushCi = (ci: CiSnapshotWire): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'ci', snapshot: ci }, '2026-01-01T00:00:00Z'))

const render = (): string => renderToStaticMarkup(<CiGlyph />)

afterEach(() => resetSessionForTest())

describe('CiGlyph — conclusion derivation from mirror.ci.state', () => {
  const CASES: Array<{
    state: CiSnapshotWire['state']
    conclusion: string
    glyph: string
  }> = [
    { state: 'green', conclusion: 'success', glyph: '✓' },
    { state: 'red', conclusion: 'failure', glyph: '✗' },
    { state: 'running', conclusion: 'in_progress', glyph: '◌' },
    { state: 'quota_exhausted', conclusion: 'quota_exhausted', glyph: '⏸' },
    { state: 'unknown', conclusion: 'unknown', glyph: '?' }
  ]

  for (const { state, conclusion, glyph } of CASES) {
    test(`state "${state}" → data-conclusion="${conclusion}", glyph ${glyph}, matching class`, () => {
      pushCi(snap({ state }))
      const html = render()
      expect(html).toContain(`data-conclusion="${conclusion}"`)
      expect(html).toContain(`CI ${glyph}`)
      expect(html).toContain(`ci-glyph--${conclusion}`)
    })
  }

  test('falls back to unknown when no ci snapshot has been mirrored yet', () => {
    const html = render()
    expect(html).toContain('data-conclusion="unknown"')
    expect(html).toContain('CI ?')
  })

  test('quota_exhausted aria-label surfaces the infra reason for SR users', () => {
    pushCi(snap({ state: 'quota_exhausted' }))
    expect(render()).toContain('aria-label="CI unavailable — GitHub Actions quota exhausted"')
  })
})

describe('CiGlyph — links to GitHub Actions via the backend-provided snapshot url (no /api/ci)', () => {
  test('href + window.open target the snapshot actions URL', () => {
    pushCi(snap({ state: 'green' }))
    expect(render()).toContain('href="https://github.com/acme/bpmux/actions"')
  })

  test('falls back to github.com when the snapshot carries no url', () => {
    pushCi(snap({ state: 'green', url: null }))
    expect(render()).toContain('href="https://github.com"')
  })
})

describe('CiGlyph — watcher-health derives from the pushed last_successful_poll_at', () => {
  test('stale poll-time flips to watcher-unhealthy class + data-state + aria-label', () => {
    pushCi(snap({ state: 'green', last_successful_poll_at: '2000-01-01T00:00:00Z' }))
    const html = render()
    expect(html).toContain('ci-glyph--watcher-unhealthy')
    expect(html).toContain('data-state="watcher-unhealthy"')
    expect(html).toContain('aria-label="CI watcher unhealthy')
  })

  test('fresh poll-time keeps the glyph healthy (tracks the conclusion)', () => {
    pushCi(snap({ state: 'green', last_successful_poll_at: new Date().toISOString() }))
    const html = render()
    expect(html).not.toContain('ci-glyph--watcher-unhealthy')
    expect(html).toContain('data-state="success"')
  })

  test('absent poll-time is not treated as unhealthy', () => {
    pushCi(snap({ state: 'green', last_successful_poll_at: null }))
    expect(render()).not.toContain('ci-glyph--watcher-unhealthy')
  })
})

describe('CiGlyph — tooltip is built from the mirrored snapshot', () => {
  test('tooltip carries branch, conclusion, title, and short sha', () => {
    pushCi(snap({ state: 'red', display_title: 'broke the build', head_sha: 'abc1234def' }))
    const html = render()
    expect(html).toContain('CI · main · failure')
    expect(html).toContain('broke the build')
    expect(html).toContain('abc1234')
  })

  test('loading copy shows until the first snapshot lands', () => {
    expect(render()).toContain('CI state loading…')
  })
})
