import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { ReworkGlyph } from './ReworkGlyph'

// #1146 slice 4 rework: visual-class falsifier per
// `feedback_visual_class_falsifier_needs_visual_assertion`. The
// substrate-side derivation of `rework_comment` is pinned by Rust
// domain tests; these tests pin that the renderer reads the field
// and emits the expected DOM shape (positive case + suppression
// case + tooltip-content case).

describe('#1146 ReworkGlyph — column-surface ↩ glyph rendering', () => {
  test('renders the glyph when rework_comment is non-null', () => {
    const html = renderToStaticMarkup(
      <ReworkGlyph reworkComment="user-story not satisfied" />
    )
    expect(html).toContain('class="board-story-rework-glyph"')
    expect(html).toContain('↩')
  })

  test('suppresses the glyph when rework_comment is null', () => {
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment={null} />)
    expect(html).toBe('')
  })

  test('suppresses the glyph when rework_comment is undefined', () => {
    const html = renderToStaticMarkup(<ReworkGlyph />)
    expect(html).toBe('')
  })

  test('suppresses the glyph when rework_comment is the empty string', () => {
    // forward-after-backward suppression: server emits null, but the
    // renderer also treats empty string as "no rework" so a
    // pathological empty payload doesn't leak a glyph with no tooltip.
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment="" />)
    expect(html).toBe('')
  })

  test('tooltip exposes the backward-edge comment via title attribute', () => {
    const comment = 'user-story not satisfied; reshape needed'
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment={comment} />)
    expect(html).toContain(`title="${comment}"`)
  })

  test('aria-label exposes the rework comment for screen readers', () => {
    const comment = 'failed accessibility scan'
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment={comment} />)
    expect(html).toContain(`aria-label="Rework: ${comment}"`)
  })

  test('renders inside a single span element with stable selector class', () => {
    // designer-lane DOM-shape contract: the glyph is a single <span>
    // with the `board-story-rework-glyph` class so the cross-surface
    // selector `[data-rework="true"] .board-story-rework-glyph`
    // (designer's filing-time desk-check selector) stays stable.
    const html = renderToStaticMarkup(
      <ReworkGlyph reworkComment="rework reason" />
    )
    expect(html.startsWith('<span')).toBe(true)
    expect(html.endsWith('</span>')).toBe(true)
    // exactly one span open + close tag
    const openCount = (html.match(/<span/g) ?? []).length
    const closeCount = (html.match(/<\/span>/g) ?? []).length
    expect(openCount).toBe(1)
    expect(closeCount).toBe(1)
  })
})
