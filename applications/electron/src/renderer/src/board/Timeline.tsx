import { format } from 'date-fns'
import { useEffect, useRef, useState } from 'react'
import {
  TIMELINE_GATE,
  useTimeline,
  type ClassifiedEvent,
  type TimelineGate
} from '../hooks/useTimeline'
import type { BoardStory } from './types'

const GATE_LABEL: Record<TimelineGate, string> = {
  filed: 'Filed',
  started: 'Started',
  delivered: 'Delivered',
  'desk-check-fail': 'Desk-check failed',
  accepted: 'Accepted',
  rejected: 'Rejected',
  closed: 'Closed',
  reopened: 'Reopened'
}

const MAJOR_GATES: ReadonlySet<TimelineGate> = new Set(Object.values(TIMELINE_GATE))

const EVENT_LOG_START = Date.parse('2026-04-01T00:00:00Z')

const labelFor = (e: ClassifiedEvent): string => {
  if (e.gate) return GATE_LABEL[e.gate]
  if (e.raw.event === 'labeled') return `Labeled: ${e.raw.label ?? '?'}`
  if (e.raw.event === 'unlabeled') return `Unlabeled: ${e.raw.label ?? '?'}`
  if (e.raw.event === 'commented') return 'Commented'
  if (e.raw.event === 'renamed') return 'Renamed'
  if (e.raw.event === 'assigned') return 'Assigned'
  if (e.raw.event === 'unassigned') return 'Unassigned'
  return e.raw.event
}

const truncate = (body: string, max = 120): string => {
  const collapsed = body.replace(/\s+/g, ' ').trim()
  if (collapsed.length <= max) return collapsed
  return `${collapsed.slice(0, max - 1)}…`
}

const renderTimelineList = (
  events: ClassifiedEvent[],
  showAll: boolean,
  story: BoardStory
): React.JSX.Element => {
  const visible = showAll
    ? events
    : events.filter((e) => e.gate !== null && MAJOR_GATES.has(e.gate))
  const createdMs = Date.parse(story.created_at ?? '')
  const preEventLog = Number.isFinite(createdMs) && createdMs < EVENT_LOG_START
  return (
    <>
      <ol className="drill-in-timeline-list" data-test="drill-in-timeline-list">
        {visible.map((e, i) => {
          const classes = [
            'drill-in-timeline-item',
            e.gate ? `drill-in-timeline-item--${e.gate}` : '',
            e.reworkGroup !== undefined ? 'drill-in-timeline-item--rework' : ''
          ]
            .filter(Boolean)
            .join(' ')
          return (
            <li
              key={`${e.raw.ts}-${i}`}
              className={classes}
              data-test="drill-in-timeline-item"
              data-gate={e.gate ?? e.raw.event}
              data-rework-group={e.reworkGroup ?? undefined}
            >
              <b className="drill-in-timeline-kind">{labelFor(e)}</b>
              <time className="drill-in-timeline-ts" dateTime={e.raw.ts}>
                {format(new Date(e.raw.ts), 'MMM d · h:mm a')}
              </time>
              {e.raw.actor && <cite className="drill-in-timeline-actor">{e.raw.actor}</cite>}
              {e.raw.body && (
                <small className="drill-in-timeline-note">{truncate(e.raw.body)}</small>
              )}
            </li>
          )
        })}
      </ol>
      {preEventLog && (
        <p className="drill-in-timeline-footer" data-test="drill-in-timeline-prelog">
          history pre-dates event log
        </p>
      )}
    </>
  )
}

interface Props {
  story: BoardStory
  apiBase: string | undefined
  usingMock: boolean
  focusTimeline?: boolean
}

const Timeline = ({ story, apiBase, usingMock, focusTimeline }: Props): React.JSX.Element => {
  const [showAll, setShowAll] = useState<boolean>(false)
  const timeline = useTimeline({
    apiBase,
    usingMock,
    storyNumber: story.number
  })
  const detailsRef = useRef<HTMLDetailsElement | null>(null)

  useEffect(() => {
    if (!focusTimeline) return
    const el = detailsRef.current
    if (!el) return
    el.open = true
    window.requestAnimationFrame(() => {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    })
  }, [focusTimeline])

  return (
    <details
      className="drill-in-timeline"
      data-test="drill-in-timeline"
      aria-label="Timeline"
      ref={detailsRef}
    >
      <summary className="drill-in-timeline-summary">
        <h4>Timeline{timeline.events ? ` (${timeline.events.length})` : ''}</h4>
      </summary>
      {timeline.loading && timeline.events === null ? (
        <p className="drill-in-timeline-empty">loading…</p>
      ) : timeline.events && timeline.events.length > 0 ? (
        <>
          <section className="drill-in-timeline-controls">
            <button
              type="button"
              className="drill-in-timeline-toggle"
              data-test="drill-in-timeline-toggle"
              aria-pressed={showAll}
              onClick={() => setShowAll((v) => !v)}
            >
              {showAll ? 'Show major gates only' : 'Show all events'}
            </button>
          </section>
          {renderTimelineList(timeline.events, showAll, story)}
        </>
      ) : (
        <p className="drill-in-timeline-empty">no events yet</p>
      )}
    </details>
  )
}

export default Timeline
