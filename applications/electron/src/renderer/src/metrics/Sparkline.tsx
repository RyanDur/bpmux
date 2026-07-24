import './Sparkline.css'

export type WipPoint = {
  date: string
  wip_count: number
}

const VIEW_W = 100
const VIEW_H = 30
const HISTORY_TABLE_ID = 'sparkline-wip-history'

type ScaledPoint = {
  x: number
  y: number
}

const scale = (history: WipPoint[]): ScaledPoint[] => {
  const counts = history.map((p) => p.wip_count)
  const max = Math.max(...counts)
  const min = Math.min(...counts)
  const span = max - min
  return history.map((p, i) => ({
    x: (i / (history.length - 1)) * VIEW_W,
    y: span === 0 ? VIEW_H / 2 : VIEW_H - ((p.wip_count - min) / span) * VIEW_H
  }))
}

const trendWord = (history: WipPoint[]): string => {
  const first = history[0].wip_count
  const last = history[history.length - 1].wip_count
  if (last > first) return 'climbing'
  if (last < first) return 'falling'
  return 'steady'
}

const Sparkline = ({ history }: { history: WipPoint[] }): React.JSX.Element => {
  if (history.length < 2) {
    return <p className="sparkline-empty">no data yet</p>
  }

  const current = history[history.length - 1].wip_count
  const label = `WIP trend: ${current} ${current === 1 ? 'item' : 'items'}, ${trendWord(history)} over ${history.length} days`
  const scaled = scale(history)
  const points = scaled.map((p) => `${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ')
  const endpoint = scaled[scaled.length - 1]

  return (
    <figure className="sparkline">
      <svg
        className="sparkline-svg"
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={label}
        aria-describedby={HISTORY_TABLE_ID}
      >
        <polyline className="sparkline-line" pathLength={100} points={points} />
        <circle className="sparkline-dot" cx={endpoint.x} cy={endpoint.y} r={2} />
      </svg>
      <table id={HISTORY_TABLE_ID} className="sr-only">
        <caption>WIP history</caption>
        <thead>
          <tr>
            <th scope="col">Date</th>
            <th scope="col">WIP</th>
          </tr>
        </thead>
        <tbody>
          {history.map((p) => (
            <tr key={p.date}>
              <td>{p.date}</td>
              <td>{p.wip_count}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </figure>
  )
}

export default Sparkline
