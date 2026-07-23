import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import { ReworkGlyph } from './ReworkGlyph'

describe('ReworkGlyph — column-surface ↩ glyph rendering', () => {
  test('renders the glyph when rework_comment is non-null', () => {
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment="user-story not satisfied" />)
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

  test('renders inside a single <i> element with stable selector class', () => {
    const html = renderToStaticMarkup(<ReworkGlyph reworkComment="rework reason" />)
    expect(html.startsWith('<i')).toBe(true)
    expect(html.endsWith('</i>')).toBe(true)
    const openCount = (html.match(/<i[\s>]/g) ?? []).length
    const closeCount = (html.match(/<\/i>/g) ?? []).length
    expect(openCount).toBe(1)
    expect(closeCount).toBe(1)
  })
})
