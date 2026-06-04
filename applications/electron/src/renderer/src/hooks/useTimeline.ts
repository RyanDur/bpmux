import { useCallback, useEffect, useState } from 'react'
import { describeHttpError, http } from '../api/http'

export interface TimelineEvent {
  event: string
  actor: string
  ts: string
  label?: string
  body?: string
}

export const TIMELINE_GATE = {
  Filed: 'filed',
  Started: 'started',
  Delivered: 'delivered',
  DeskCheckFail: 'desk-check-fail',
  Accepted: 'accepted',
  Rejected: 'rejected',
  Closed: 'closed',
  Reopened: 'reopened'
} as const

export type TimelineGate = (typeof TIMELINE_GATE)[keyof typeof TIMELINE_GATE]

export interface ClassifiedEvent {
  raw: TimelineEvent
  gate: TimelineGate | null
  reworkGroup?: number
}

interface UseTimelineArgs {
  apiBase: string | undefined
  authToken: string | undefined
  storyNumber: number | null
  boardInvalidateTick: number
}

export interface UseTimelineResult {
  events: ClassifiedEvent[] | null
  loading: boolean
  error: string | null
  refetch: () => Promise<void>
}

export const classifyEvent = (e: TimelineEvent): TimelineGate | null => {
  if (e.event === 'filed') return TIMELINE_GATE.Filed
  if (e.event === 'closed') return TIMELINE_GATE.Closed
  if (e.event === 'reopened') return TIMELINE_GATE.Reopened
  if (e.event === 'labeled') {
    switch (e.label) {
      case 'in progress':
        return TIMELINE_GATE.Started
      case 'delivered':
        return TIMELINE_GATE.Delivered
      case 'desk-check-fail':
        return TIMELINE_GATE.DeskCheckFail
      case 'accepted':
        return TIMELINE_GATE.Accepted
      case 'rejected':
        return TIMELINE_GATE.Rejected
      default:
        return null
    }
  }
  return null
}

const markReworkCycles = (events: ClassifiedEvent[]): ClassifiedEvent[] => {
  const out = events.map((e) => ({ ...e }))
  let cycle = 0
  let inCycle = false
  for (const e of out) {
    if (!inCycle && e.gate === TIMELINE_GATE.DeskCheckFail) {
      cycle += 1
      inCycle = true
      e.reworkGroup = cycle
      continue
    }
    if (inCycle) {
      e.reworkGroup = cycle
      if (
        e.gate === TIMELINE_GATE.Accepted ||
        e.gate === TIMELINE_GATE.Closed ||
        e.gate === TIMELINE_GATE.Rejected
      ) {
        inCycle = false
      }
    }
  }
  return out
}

export const useTimeline = ({
  apiBase,
  authToken,
  storyNumber,
  boardInvalidateTick
}: UseTimelineArgs): UseTimelineResult => {
  const [events, setEvents] = useState<ClassifiedEvent[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const loading =
    events === null &&
    error === null &&
    storyNumber !== null &&
    apiBase !== undefined &&
    authToken !== undefined

  const refetch = useCallback(async (): Promise<void> => {
    if (!apiBase || !authToken || storyNumber === null) return
    try {
      const res = await fetch(`${apiBase}/api/stories/${storyNumber}/timeline`, {
        headers: { Authorization: `Bearer ${authToken}` }
      })
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
    }
  }, [apiBase, authToken, storyNumber])

  useEffect(() => {
    if (storyNumber === null || !apiBase || !authToken) {
      return
    }
    const controller = new AbortController()
    http
      .get<TimelineEvent[]>(
        { apiBase, authToken },
        `/api/stories/${storyNumber}/timeline`,
        controller.signal
      )
      .onSuccess((raw) => {
        if (controller.signal.aborted) return
        const classified: ClassifiedEvent[] = raw.map((r) => ({
          raw: r,
          gate: classifyEvent(r)
        }))
        setEvents(markReworkCycles(classified))
        setError(null)
      })
      .onFailure((err) => {
        if (controller.signal.aborted) return
        setError(describeHttpError(err))
      })
    return () => controller.abort()
  }, [storyNumber, apiBase, authToken])

  useEffect(() => {
    if (boardInvalidateTick === 0) return
    const id = setTimeout(() => void refetch(), 0)
    return () => clearTimeout(id)
  }, [boardInvalidateTick, refetch])

  return { events, loading, error, refetch }
}
