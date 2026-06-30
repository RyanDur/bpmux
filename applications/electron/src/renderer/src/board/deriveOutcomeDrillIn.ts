import type { IterationReadiness } from './types'
import type { RoadmapIteration } from './selectRoadmap'
import { CHECK_ORDER, firstUnmetLabel, nothingDone, unmetCount } from './iterationReadiness'

export type ConveneCheckState = 'met' | 'gap' | 'pending'

export interface ConveneCheck {
  key: keyof IterationReadiness
  label: string
  state: ConveneCheckState
}

export type DrillIterationState = 'ready' | 'next' | 'gap' | 'not-started'

export interface DrillIteration {
  id: number
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

export const deriveOutcomeDrillIn = (outcomeIterations: RoadmapIteration[]): OutcomeDrillIn => {
  const building = [...outcomeIterations].sort((a, b) => a.position - b.position)

  const nextIndex = building.findIndex(
    (iteration) => !nothingDone(iteration.readiness) && unmetCount(iteration.readiness) > 0
  )

  const iterations: DrillIteration[] = building.map((iteration, index) => {
    const readiness = iteration.readiness
    const gapCount = unmetCount(readiness)
    const state: DrillIterationState = nothingDone(readiness)
      ? 'not-started'
      : gapCount === 0
        ? 'ready'
        : index === nextIndex
          ? 'next'
          : 'gap'
    return {
      id: iteration.id,
      name: iteration.name,
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
  const then = building[nextIndex + 1]
  return {
    iterations,
    convene: {
      nextIterationName: next.name,
      nextGapCount: unmetCount(next.readiness),
      activeGapLabel: firstUnmetLabel(next.readiness),
      thenIterationName: then?.name ?? null
    }
  }
}
