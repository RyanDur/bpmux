import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import CiGlyph from './CiGlyph'
import type { CiSnapshotWire } from '../transport/types'

const snap = (state: CiSnapshotWire['state']): CiSnapshotWire => ({
  state,
  run_id: 7,
  head_sha: 'abc1234',
  display_title: 'test commit'
})

describe('CiGlyph — mirror.ci.state derivation (slice 5b canonical archetype)', () => {
  test('renders ✓ + data-conclusion="success" when ciSnapshot.state === "green"', () => {
    const html = renderToStaticMarkup(<CiGlyph ciSnapshot={snap('green')} />)
    expect(html).toContain('data-conclusion="success"')
    expect(html).toContain('CI ✓')
    expect(html).toContain('ci-glyph--success')
  })

  test('renders ✗ + data-conclusion="failure" when ciSnapshot.state === "red"', () => {
    const html = renderToStaticMarkup(<CiGlyph ciSnapshot={snap('red')} />)
    expect(html).toContain('data-conclusion="failure"')
    expect(html).toContain('CI ✗')
    expect(html).toContain('ci-glyph--failure')
  })

  test('falls back to REST conclusion path when ciSnapshot is null (renders unknown until REST resolves)', () => {
    const html = renderToStaticMarkup(<CiGlyph ciSnapshot={null} />)
    expect(html).toContain('data-conclusion="unknown"')
    expect(html).toContain('CI ?')
  })
})
