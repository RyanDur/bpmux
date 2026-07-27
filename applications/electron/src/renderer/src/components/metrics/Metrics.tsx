import { useEffect, useState } from 'react'
import { describeHttpError, http } from '@transport/http'
import Sparkline, { type WipPoint } from './Sparkline'
import './Metrics.css'

type Props = {
  apiBase: string | undefined
  authToken: string | undefined
  onClose: () => void
  open: boolean
}

type VelocityPayload = {
  window_days: number
  delivered_in_window: number
  delivered_per_day: number
  median_pickup_to_deliver_hours: number | null
  median_deliver_to_accept_hours: number | null
  rework_rate: number | null
  sparse: boolean
  note: string | null
  wip_history: WipPoint[]
  dora?: {
    deployment_frequency_per_day: number
    change_failure_rate: number | null
    mttr_hours: number | null
  }
}

const currentWip = (history: WipPoint[]): number =>
  history.length > 0 ? history[history.length - 1].wip_count : 0

const formatHours = (h: number | null): string => {
  if (h === null) return '—'
  if (h < 1) return `${Math.round(h * 60)}m`
  if (h < 24) return `${h.toFixed(1)}h`
  return `${(h / 24).toFixed(1)}d`
}

const formatPercent = (p: number | null): string => {
  if (p === null) return '—'
  return `${Math.round(p * 100)}%`
}

const Bar = ({
  value,
  max,
  color
}: {
  value: number
  max: number
  color: string
}): React.JSX.Element => {
  const width = Math.min(100, (value / max) * 100)
  return (
    <svg className="metrics-bar" viewBox="0 0 100 8" preserveAspectRatio="none" aria-hidden="true">
      <rect x="0" y="0" width="100" height="8" fill="var(--bg-highlighted)" />
      <rect x="0" y="0" width={width} height="8" fill={color} />
    </svg>
  )
}

const Metrics = ({ apiBase, authToken, onClose, open }: Props): React.JSX.Element => {
  const [data, setData] = useState<VelocityPayload | null>(null)
  const [fetchError, setFetchError] = useState<string | null>(null)
  const error = open && !apiBase ? 'backend not reachable' : fetchError

  useEffect(() => {
    if (!open || !apiBase) return
    return http
      .get<VelocityPayload>({ apiBase, authToken }, '/api/metrics')
      .onSuccess(setData)
      .onFailure((err) => setFetchError(describeHttpError(err))).cancel
  }, [open, apiBase, authToken])

  return (
    <dialog
      open={open}
      className="metrics"
      aria-label="Metrics"
      data-test="metrics-overlay"
      onClose={onClose}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault()
          onClose()
        }
      }}
    >
      <header className="metrics-header">
        <h2>Velocity + DORA</h2>
        <button
          type="button"
          className="metrics-close"
          onClick={onClose}
          aria-label="Close metrics"
        >
          ✕
        </button>
      </header>

      {error !== null && (
        <p className="metrics-error" role="alert">
          Couldn&apos;t load metrics: {error}
        </p>
      )}

      {data !== null && (
        <section className="metrics-grid">
          <MetricTile
            label="In progress (WIP)"
            value={String(currentWip(data.wip_history))}
            bar={<Sparkline history={data.wip_history} />}
          />
          <MetricTile
            label={`Delivered / day (${data.window_days}d)`}
            value={data.delivered_per_day.toFixed(2)}
            bar={<Bar value={data.delivered_per_day} max={5} color="var(--accent-success)" />}
          />
          <MetricTile label="Delivered in window" value={String(data.delivered_in_window)} />
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
          {data.dora && (
            <>
              <MetricTile
                label={`Deploys / day (${data.window_days}d)`}
                value={data.dora.deployment_frequency_per_day.toFixed(2)}
                bar={
                  <Bar
                    value={data.dora.deployment_frequency_per_day}
                    max={5}
                    color="var(--accent-success)"
                  />
                }
              />
              <MetricTile
                label="Change failure rate"
                value={formatPercent(data.dora.change_failure_rate)}
                bar={
                  <Bar
                    value={(data.dora.change_failure_rate ?? 0) * 100}
                    max={100}
                    color={
                      data.dora.change_failure_rate && data.dora.change_failure_rate > 0.2
                        ? 'var(--accent-warning)'
                        : 'var(--accent-success)'
                    }
                  />
                }
              />
              <MetricTile
                label="MTTR (red → green)"
                value={formatHours(data.dora.mttr_hours)}
                bar={<Bar value={data.dora.mttr_hours ?? 0} max={48} color="var(--accent-info)" />}
              />
            </>
          )}
          {data.sparse && (
            <p className="metrics-note">
              <small>{data.note ?? 'Sparse data window.'}</small>
            </p>
          )}
        </section>
      )}
    </dialog>
  )
}

const MetricTile = ({
  label,
  value,
  bar
}: {
  label: string
  value: string
  bar?: React.JSX.Element
}): React.JSX.Element => (
  <dl className="metrics-tile">
    <dt>{label}</dt>
    <dd>
      <strong className="metrics-value">{value}</strong>
      {bar}
    </dd>
  </dl>
)

export default Metrics
