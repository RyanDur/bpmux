import { afterEach, describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import CiGlyph from './CiGlyph'
import { dispatchSession, resetSessionForTest } from '../transport/session/store'
import { mirrorDeltaReceived } from '../transport/session/actions'
import type { CiSnapshotWire } from '../transport/types'

const snap = (state: CiSnapshotWire['state']): CiSnapshotWire => ({
  state,
  run_id: 7,
  head_sha: 'abc1234',
  display_title: 'test commit'
})

const pushCi = (ci: CiSnapshotWire): void =>
  dispatchSession(mirrorDeltaReceived({ domain: 'ci', snapshot: ci }, '2026-01-01T00:00:00Z'))

afterEach(() => resetSessionForTest())

describe('CiGlyph — mirror.ci.state derivation (slice 5b canonical archetype)', () => {
  test('renders ✓ + data-conclusion="success" when ci state === "green"', () => {
    pushCi(snap('green'))
    const html = renderToStaticMarkup(<CiGlyph />)
    expect(html).toContain('data-conclusion="success"')
    expect(html).toContain('CI ✓')
    expect(html).toContain('ci-glyph--success')
  })

  test('renders ✗ + data-conclusion="failure" when ci state === "red"', () => {
    pushCi(snap('red'))
    const html = renderToStaticMarkup(<CiGlyph />)
    expect(html).toContain('data-conclusion="failure"')
    expect(html).toContain('CI ✗')
    expect(html).toContain('ci-glyph--failure')
  })

  test('falls back to unknown when no ci snapshot has been mirrored yet', () => {
    const html = renderToStaticMarkup(<CiGlyph />)
    expect(html).toContain('data-conclusion="unknown"')
    expect(html).toContain('CI ?')
  })
})
