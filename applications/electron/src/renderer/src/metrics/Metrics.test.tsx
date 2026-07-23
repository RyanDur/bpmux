import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Metrics from './Metrics'

const baseProps = {
  apiBase: undefined,
  authToken: undefined,
  onClose: (): void => {}
}

describe('Metrics dialog open/close is declarative (no ref)', () => {
  test('open=true reflects the open attribute on the <dialog> in markup', () => {
    const html = renderToStaticMarkup(<Metrics {...baseProps} open={true} />)
    expect(html).toMatch(/<dialog\b[^>]*\bopen\b/)
    expect(html).toContain('Velocity + DORA')
    expect(html).toContain('aria-label="Close metrics"')
  })

  test('open=false omits the open attribute (dialog closed) declaratively', () => {
    const html = renderToStaticMarkup(<Metrics {...baseProps} open={false} />)
    expect(html).not.toMatch(/<dialog\b[^>]*\bopen\b/)
  })
})
