import { useCallback, useEffect, useState } from 'react'
import { describeHttpError, http } from '../api/http'

export interface TimelineEvent {
  event: string
  actor: string
  ts: string
  label?: string
  body?: string
}

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

const markReworkCycles = (events: ClassifiedEvent[]): ClassifiedEvent[] => {
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
