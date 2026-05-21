import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Comments from './Comments'

const baseProps = {
  storyNumber: 999,
  apiBase: undefined,
  authToken: undefined,
  usingMock: false,
  messages: [],
  onError: () => {}
}

describe('#1355 Comments independence', () => {
  test('renders standalone (substrate-axis) — no DrillIn.tsx parent required', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toMatch(/<section[^>]*class="drill-in-comments"/)
    expect(html).toContain('aria-label="Comments"')
  })

  test('renders Comments header heading', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('<h4>Comments')
  })

  test('renders empty state on initial mount when no backend (apiBase=undefined)', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('drill-in-comments-empty')
    expect(html).toContain('no comments yet')
  })

  test('renders the comment submit form regardless of comments state', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    expect(html).toContain('data-test="drill-in-comment-input"')
    expect(html).toContain('data-test="drill-in-comment-send"')
    expect(html).toContain('Cmd+Enter to send')
  })

  test('submit button is disabled when draft is empty (initial state)', () => {
    const html = renderToStaticMarkup(<Comments {...baseProps} />)
    const sendButtonMatch = html.match(/<button[^>]*data-test="drill-in-comment-send"[^>]*>/)
    expect(sendButtonMatch).not.toBeNull()
    expect(sendButtonMatch?.[0]).toContain('disabled')
  })

  test('renders standalone for different stories with distinct storyNumber prop', () => {
    const a = renderToStaticMarkup(<Comments {...baseProps} storyNumber={101} />)
    const b = renderToStaticMarkup(<Comments {...baseProps} storyNumber={102} />)
    expect(a).toContain('drill-in-comments')
    expect(b).toContain('drill-in-comments')
  })
})
