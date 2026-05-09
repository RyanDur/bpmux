import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import IterationMarker from './IterationMarker'
import type { IterationMarker as IterationMarkerData } from '../hooks/useIterationMarkers'

const marker = (overrides: Partial<IterationMarkerData> = {}): IterationMarkerData => ({
  id: 1,
  theme_name: 'Orion',
  position: 0,
  created_at: '2026-05-09T19:00:00Z',
  ...overrides
})

describe('#1249 IterationMarker — DOM-shape + visual differentiation per variant', () => {
  test('renders as <li role="separator"> with theme-name visible', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker()} variant="next" />
    )
    expect(html).toContain('<li')
    expect(html).toContain('role="separator"')
    expect(html).toContain('Orion')
  })

  test('next variant gets data-variant="next" + class modifier', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker()} variant="next" />
    )
    expect(html).toContain('data-variant="next"')
    expect(html).toContain('board-iteration-marker--next')
  })

  test('future variant gets data-variant="future" + class modifier', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker()} variant="future" />
    )
    expect(html).toContain('data-variant="future"')
    expect(html).toContain('board-iteration-marker--future')
  })

  test('aria-label includes human-readable variant + theme-name (NOT raw type:iteration-marker)', () => {
    const next = renderToStaticMarkup(
      <IterationMarker marker={marker({ theme_name: 'Lyra' })} variant="next" />
    )
    expect(next).toContain('aria-label="next iteration: Lyra"')
    expect(next).not.toContain('type: iteration-marker')

    const future = renderToStaticMarkup(
      <IterationMarker marker={marker({ theme_name: 'Cygnus' })} variant="future" />
    )
    expect(future).toContain('aria-label="future iteration: Cygnus"')
  })

  test('data-test selector includes marker id for stable e2e targeting', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker({ id: 42 })} variant="next" />
    )
    expect(html).toContain('data-test="iteration-marker-42"')
  })

  test('renders ★ glyph for visual differentiation', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker()} variant="next" />
    )
    expect(html).toContain('★')
    expect(html).toContain('class="board-iteration-marker-glyph"')
  })

  test('theme-name appears in <b> for emphasis', () => {
    const html = renderToStaticMarkup(
      <IterationMarker marker={marker({ theme_name: 'Cassiopeia' })} variant="next" />
    )
    expect(html).toMatch(/<b[^>]*>Cassiopeia<\/b>/)
  })
})
