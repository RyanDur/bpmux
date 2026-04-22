import { useEffect, useState } from 'react'
import './Metrics.css'

interface Props {
  apiBase: string | undefined
  authToken: string | undefined
  onClose: () => void
}

interface VelocityPayload {
  window_days: number
  delivered_in_window: number
  delivered_per_day: number
  median_pickup_to_deliver_hours: number | null
  median_deliver_to_accept_hours: number | null
  rework_rate: number | null
  sparse: boolean
  note: string | null
}

function formatHours(h: number | null): string {
  if (h === null) return '—'
  if (h < 1) return `${Math.round(h * 60)}m`
  if (h < 24) return `${h.toFixed(1)}h`
  return `${(h / 24).toFixed(1)}d`
}

function formatPercent(p: number | null): string {
  if (p === null) return '—'
  return `${Math.round(p * 100)}%`
}

function Bar({ value, max, color }: { value: number; max: number; color: string }): React.JSX.Element {
  const width = Math.min(100, (value / max) * 100)
  return (
    <svg className="metrics-bar" viewBox="0 0 100 8" preserveAspectRatio="none" aria-hidden="true">
      <rect x="0" y="0" width="100" height="8" fill="var(--bg-highlighted)" />
      <rect x="0" y="0" width={width} height="8" fill={color} />
    </svg>
  )
}

function Metrics({ apiBase, authToken, onClose }: Props): React.JSX.Element {
  const [data, setData] = useState<VelocityPayload | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!apiBase) {
      setError('backend not reachable')
      return
    }
    const controller = new AbortController()
    fetch(`${apiBase}/api/metrics`, {
      headers: { Authorization: `Bearer ${authToken ?? ''}` },
      signal: controller.signal
    })
      .then(async (res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return (await res.json()) as VelocityPayload
      })
      .then((json) => setData(json))
      .catch((err) => {
        if ((err as Error)?.name === 'AbortError') return
        setError(err instanceof Error ? err.message : String(err))
      })
    return () => controller.abort()
  }, [apiBase, authToken])

  const handleKeyDown = (event: React.KeyboardEvent<HTMLElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault()
      onClose()
    }
  }

  return (
    <section
      className="metrics"
      aria-label="Metrics"
      tabIndex={0}
      onKeyDown={handleKeyDown}
      data-test="metrics-overlay"
    >
      <header className="metrics-header">
        <h2>Velocity + DORA</h2>
        <button type="button" className="metrics-close" onClick={onClose} aria-label="Close metrics">
          ✕
        </button>
      </header>

      {error !== null && <p className="metrics-error" role="alert">Couldn&apos;t load metrics: {error}</p>}

      {data !== null && (
        <dl className="metrics-grid">
          <MetricTile
            label={`Delivered / day (${data.window_days}d)`}
            value={data.delivered_per_day.toFixed(2)}
            bar={<Bar value={data.delivered_per_day} max={5} color="var(--accent-success)" />}
          />
          <MetricTile
            label="Delivered in window"
            value={String(data.delivered_in_window)}
          />
          <MetricTile
            label="Median pickup → deliver"
            value={formatHours(data.median_pickup_to_deliver_hours)}
            bar={
              <Bar
                value={data.median_pickup_to_deliver_hours ?? 0}
                max={48}
                color="var(--accent-info)"
              />
            }
          />
          <MetricTile
            label="Median deliver → accept"
            value={formatHours(data.median_deliver_to_accept_hours)}
            bar={
              <Bar
                value={data.median_deliver_to_accept_hours ?? 0}
                max={48}
                color="var(--accent-info)"
              />
            }
          />
          <MetricTile
            label="Rework rate"
            value={formatPercent(data.rework_rate)}
            bar={
              <Bar
                value={(data.rework_rate ?? 0) * 100}
                max={100}
                color={
                  data.rework_rate && data.rework_rate > 0.2
                    ? 'var(--accent-warning)'
                    : 'var(--accent-success)'
                }
              />
            }
          />
          {data.sparse && (
            <p className="metrics-note">
              <small>{data.note ?? 'Sparse data window.'}</small>
            </p>
          )}
        </dl>
      )}
    </section>
  )
}

function MetricTile({
  label,
  value,
  bar
}: {
  label: string
  value: string
  bar?: React.JSX.Element
}): React.JSX.Element {
  // HTML5 spec restricts <dl>'s permitted content to <dt>, <dd>, and
  // <div> grouping wrappers — <section> inside <dl> is invalid and
  // causes assistive tech to mispair term/definition in some readers
  // (#338 follow-up).
  return (
    <div className="metrics-tile">
      <dt>{label}</dt>
      <dd>
        <strong className="metrics-value">{value}</strong>
        {bar}
      </dd>
    </div>
  )
}

export default Metrics
