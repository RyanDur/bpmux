import { useCallback, useEffect, useRef, useState } from 'react'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'

// Raw timeline event as returned by GET /api/stories/:number/timeline.
// The backend forwards GitHub's issue-events list + a synthetic
// "filed" record for the issue's own creation. Classification into
// lifecycle gates (started / delivered / accepted / rejected / closed)
// happens here on the renderer — the mapping is a product concern
// (bpmux's label vocabulary) that shouldn't live in the API layer.
export interface TimelineEvent {
  event: string
  actor: string
  ts: string
  label?: string
  body?: string
}

// The classified gate kinds we surface in the default "major gates"
// filter. Any event that isn't one of these falls under `labeled-
// other` / `commented` / `other` and hides unless the user toggles
// "show all".
export type TimelineGate =
  | 'filed'
  | 'started'
  | 'delivered'
  | 'desk-check-fail'
  | 'accepted'
  | 'rejected'
  | 'closed'
  | 'reopened'

export interface ClassifiedEvent {
  raw: TimelineEvent
  gate: TimelineGate | null
  // Rework cycles are contiguous runs starting with a desk-check-fail
  // that end when the next accepted / closed / rejected gate fires.
  // When present, groups the event visually with its siblings in the
  // same cycle so readers see the loop as a single iteration rather
  // than a flat list of gate events.
  reworkGroup?: number
}

interface UseTimelineArgs {
  apiBase: string | undefined
  authToken: string | undefined
  storyNumber: number | null
  messages: ChatMessage[]
}

export interface UseTimelineResult {
  events: ClassifiedEvent[] | null
  loading: boolean
  error: string | null
  refetch: () => Promise<void>
}

// Map a raw issue-event + label name to a lifecycle gate. Mirrors the
// label vocabulary the Rust bridge writes in
// `tools_stories::{start_story, deliver_story, accept_story,
// reject_story}`. Consumers that want the filing + closure anchors
// get those from the synthetic `filed` + closed events directly.
export function classifyEvent(e: TimelineEvent): TimelineGate | null {
  if (e.event === 'filed') return 'filed'
  if (e.event === 'closed') return 'closed'
  if (e.event === 'reopened') return 'reopened'
  if (e.event === 'labeled') {
    switch (e.label) {
      case 'in progress':
        return 'started'
      case 'delivered':
        return 'delivered'
      case 'desk-check-fail':
        return 'desk-check-fail'
      case 'accepted':
        return 'accepted'
      case 'rejected':
        return 'rejected'
      default:
        return null
    }
  }
  return null
}

// Annotate desk-check-fail → … → (accepted|closed|rejected) runs with
// a rework group id. The loop's start is the desk-check-fail itself;
// the close is the next terminal gate. Events without a group stay
// ungrouped (reworkGroup: undefined).
function markReworkCycles(events: ClassifiedEvent[]): ClassifiedEvent[] {
  const out = events.map((e) => ({ ...e }))
  let cycle = 0
  let inCycle = false
  for (const e of out) {
    if (!inCycle && e.gate === 'desk-check-fail') {
      cycle += 1
      inCycle = true
      e.reworkGroup = cycle
      continue
    }
    if (inCycle) {
      e.reworkGroup = cycle
      if (e.gate === 'accepted' || e.gate === 'closed' || e.gate === 'rejected') {
        inCycle = false
      }
    }
  }
  return out
}

export function useTimeline({
  apiBase,
  authToken,
  storyNumber,
  messages
}: UseTimelineArgs): UseTimelineResult {
  const [events, setEvents] = useState<ClassifiedEvent[] | null>(null)
  const [loading, setLoading] = useState<boolean>(false)
  const [error, setError] = useState<string | null>(null)
  // Track message count across renders so we only refetch on new
  // board-invalidate markers (not on the hydration backfill). Mirrors
  // the pattern DrillIn uses for comment refetches.
  const seenMessageCountRef = useRef<number>(messages.length)

  const refetch = useCallback(async (): Promise<void> => {
    if (!apiBase || !authToken || storyNumber === null) return
    setLoading(true)
    try {
      const res = await fetch(
        `${apiBase}/api/stories/${storyNumber}/timeline`,
        {
          headers: { Authorization: `Bearer ${authToken}` }
        }
      )
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const raw = (await res.json()) as TimelineEvent[]
      const classified: ClassifiedEvent[] = raw.map((r) => ({
        raw: r,
        gate: classifyEvent(r)
      }))
      setEvents(markReworkCycles(classified))
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [apiBase, authToken, storyNumber])

  useEffect(() => {
    if (storyNumber === null) {
      setEvents(null)
      return
    }
    void refetch()
  }, [storyNumber, refetch])

  useEffect(() => {
    const prev = seenMessageCountRef.current
    seenMessageCountRef.current = messages.length
    if (messages.length <= prev) return
    const newMessages = messages.slice(prev)
    const hasInvalidate = newMessages.some(
      (m) => m.text === BOARD_INVALIDATE_MARKER
    )
    if (hasInvalidate) void refetch()
  }, [messages, refetch])

  return { events, loading, error, refetch }
}
