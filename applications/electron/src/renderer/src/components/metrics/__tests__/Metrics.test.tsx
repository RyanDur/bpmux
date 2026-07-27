import { describe, expect, test } from 'vitest'
import { renderToStaticMarkup } from 'react-dom/server'
import Metrics from '../Metrics'

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

describe('DORA tiles derive from the payload dora block', () => {
  const payload = {
    window_days: 14,
    delivered_in_window: 3,
    delivered_per_day: 0.21,
    median_pickup_to_deliver_hours: 5,
    median_deliver_to_accept_hours: 2,
    rework_rate: 0.1,
    sparse: false,
    note: null,
    wip_history: [],
    dora: {
      deployment_frequency_per_day: 1.5,
      change_failure_rate: 0.25,
      mttr_hours: 6.4
    }
  }

  test('a payload with dora renders the three tiles with honest formatting', async () => {
    const originalFetch = globalThis.fetch
    globalThis.fetch = (async () =>
      new Response(JSON.stringify(payload), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      })) as typeof fetch
    try {
      const { render, screen } = await import('@testing-library/react')
      render(<Metrics apiBase="http://127.0.0.1:1" authToken="t" onClose={() => {}} open={true} />)
      expect(await screen.findByText('Deploys / day (14d)')).toBeTruthy()
      expect(screen.getByText('1.50')).toBeTruthy()
      expect(screen.getByText('Change failure rate')).toBeTruthy()
      expect(screen.getByText('25%')).toBeTruthy()
      expect(screen.getByText('MTTR (red → green)')).toBeTruthy()
      expect(screen.getByText('6.4h')).toBeTruthy()
    } finally {
      globalThis.fetch = originalFetch
    }
  })

  test('null CFR and MTTR render as dashes, never zeros', async () => {
    const originalFetch = globalThis.fetch
    globalThis.fetch = (async () =>
      new Response(
        JSON.stringify({
          ...payload,
          dora: {
            deployment_frequency_per_day: 0,
            change_failure_rate: null,
            mttr_hours: null
          }
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      )) as typeof fetch
    try {
      const { render, screen } = await import('@testing-library/react')
      render(<Metrics apiBase="http://127.0.0.1:1" authToken="t" onClose={() => {}} open={true} />)
      expect(await screen.findByText('Change failure rate')).toBeTruthy()
      expect(screen.getAllByText('—').length).toBeGreaterThanOrEqual(2)
    } finally {
      globalThis.fetch = originalFetch
    }
  })
})
