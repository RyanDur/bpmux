import type { BoardStory, IterationReadiness } from './types'
import { storyTypeFromLabels } from './types'
import {
  CHECK_ORDER,
  firstUnmetLabel,
  nothingDone,
  readinessOf,
  unmetCount
} from './iterationReadiness'

export type ConveneCheckState = 'met' | 'gap' | 'pending'

export interface ConveneCheck {
  key: keyof IterationReadiness
  label: string
  state: ConveneCheckState
}

export type DrillIterationState = 'ready' | 'next' | 'gap' | 'not-started'

export interface DrillIteration {
  number: number
  name: string
  order: number
  state: DrillIterationState
  gapCount: number
  checks: ConveneCheck[]
}

export interface ConveneBanner {
  nextIterationName: string
  nextGapCount: number
  activeGapLabel: string | null
  thenIterationName: string | null
}

export interface OutcomeDrillIn {
  iterations: DrillIteration[]
  convene: ConveneBanner | null
}

const checksOf = (readiness: IterationReadiness): ConveneCheck[] => {
  let activeGapTaken = false
  return CHECK_ORDER.map(({ key, label }) => {
    if (readiness[key]) return { key, label, state: 'met' }
    if (!activeGapTaken) {
      activeGapTaken = true
      return { key, label, state: 'gap' }
    }
    return { key, label, state: 'pending' }
  })
}

export const deriveOutcomeDrillIn = (
  outcomeNumber: number,
  stories: BoardStory[]
): OutcomeDrillIn => {
  const building = stories
    .filter(
      (story) =>
        storyTypeFromLabels(story.labels) === 'iteration' &&
        story.outcome_membership?.outcome_number === outcomeNumber
    )
    .sort((a, b) => (a.outcome_membership?.position ?? 0) - (b.outcome_membership?.position ?? 0))

  const nextIndex = building.findIndex((iteration) => {
    const readiness = readinessOf(iteration)
    return !nothingDone(readiness) && unmetCount(readiness) > 0
  })

  const iterations: DrillIteration[] = building.map((iteration, index) => {
    const readiness = readinessOf(iteration)
    const gapCount = unmetCount(readiness)
    const state: DrillIterationState = nothingDone(readiness)
      ? 'not-started'
      : gapCount === 0
        ? 'ready'
        : index === nextIndex
          ? 'next'
          : 'gap'
    return {
      number: iteration.number,
      name: iteration.title,
      order: index + 1,
      state,
      gapCount,
      checks: state === 'not-started' ? [] : checksOf(readiness)
    }
  })

  if (nextIndex === -1) {
    return { iterations, convene: null }
  }

  const next = building[nextIndex]
  const nextReadiness = readinessOf(next)
  const then = building[nextIndex + 1]
  return {
    iterations,
    convene: {
      nextIterationName: next.title,
      nextGapCount: unmetCount(nextReadiness),
      activeGapLabel: firstUnmetLabel(nextReadiness),
      thenIterationName: then?.title ?? null
    }
  }
}
