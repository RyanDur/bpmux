import { useEffect, useState } from 'react'
import './CiGlyph.css'

interface CiStateReport {
  conclusion: 'success' | 'failure' | 'in_progress' | 'unknown'
  last_run_id: number | null
  branch: string
  url: string
  head_sha: string | null
  display_title: string | null
}

const POLL_MS = 60_000

function conclusionGlyph(conclusion: string): string {
  switch (conclusion) {
    case 'success':
      return '✓'
    case 'failure':
      return '✗'
    case 'in_progress':
      return '◌'
    default:
      return '?'
  }
}

function CiGlyph(): React.JSX.Element {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [state, setState] = useState<CiStateReport | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!bridge?.apiBase) {
      setError('no backend')
      return
    }
    let cancelled = false
    const controller = new AbortController()
    const fetchOnce = async (): Promise<void> => {
      try {
        const res = await fetch(`${bridge.apiBase}/api/ci`, {
          headers: { Authorization: `Bearer ${bridge.authToken ?? ''}` },
          signal: controller.signal
        })
        if (!res.ok) {
          if (!cancelled) setError(`HTTP ${res.status}`)
          return
        }
        const json = (await res.json()) as CiStateReport
        if (!cancelled) {
          setState(json)
          setError(null)
        }
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        if (!cancelled) setError(err instanceof Error ? err.message : String(err))
      }
    }
    void fetchOnce()
    const id = setInterval(() => {
      void fetchOnce()
    }, POLL_MS)
    return () => {
      cancelled = true
      controller.abort()
      clearInterval(id)
    }
  }, [bridge?.apiBase, bridge?.authToken])

  const conclusion = state?.conclusion ?? 'unknown'
  const glyph = conclusionGlyph(conclusion)
  const url = state?.url ?? 'https://github.com'
  const tooltip = buildTooltip(state, error)

  return (
    <a
      className={`ci-glyph ci-glyph--${conclusion}`}
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        e.preventDefault()
        if (url) window.open(url, '_blank', 'noopener,noreferrer')
      }}
      title={tooltip}
      aria-label={`CI ${conclusion}`}
      data-test="ci-glyph"
    >
      CI {glyph}
    </a>
  )
}

function buildTooltip(state: CiStateReport | null, error: string | null): string {
  if (error) return `CI error: ${error}`
  if (!state) return 'CI state loading…'
  const lines = [`CI · ${state.branch} · ${state.conclusion}`]
  if (state.display_title) lines.push(state.display_title)
  if (state.head_sha) lines.push(state.head_sha.slice(0, 7))
  lines.push('(click to open on GitHub)')
  return lines.join('\n')
}

export default CiGlyph
