import { useCallback, useEffect, useMemo } from 'react'
import {
  requestTimelineHydrate,
  useBoardInvalidateTick,
  useTimelineEvents
} from '../transport/session'
import type { TimelineEventWire } from '../transport/types'
import { asStoryId, type StoryId } from '../board/types'

export type TimelineEvent = TimelineEventWire

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
  storyId: StoryId | null
}

export interface UseTimelineResult {
  events: ClassifiedEvent[] | null
  loading: boolean
  refetch: () => void
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

export const useTimeline = ({ apiBase, storyId }: UseTimelineArgs): UseTimelineResult => {
  const boardInvalidateTick = useBoardInvalidateTick()
  const offline = apiBase === undefined
  const rawEvents = useTimelineEvents(storyId ?? asStoryId(-1))

  const events = useMemo<ClassifiedEvent[] | null>(() => {
    if (offline) return []
    if (storyId === null || rawEvents === null) return null
    const classified: ClassifiedEvent[] = rawEvents.map((r) => ({
      raw: r,
      gate: classifyEvent(r)
    }))
    return markReworkCycles(classified)
  }, [offline, storyId, rawEvents])

  const loading = events === null

  const refetch = useCallback((): void => {
    if (offline || storyId === null) return
    requestTimelineHydrate(storyId)
  }, [offline, storyId])

  useEffect(() => {
    if (offline || storyId === null) return
    requestTimelineHydrate(storyId)
  }, [offline, storyId, boardInvalidateTick])

  return { events, loading, refetch }
}
