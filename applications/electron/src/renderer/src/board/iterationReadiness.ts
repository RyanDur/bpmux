import type { BoardIterationData } from './selectIterations'
import type { IterationReadiness } from './types'

export const CHECK_ORDER: ReadonlyArray<{ key: keyof IterationReadiness; label: string }> = [
  { key: 'drafted', label: 'drafted' },
  { key: 'stories_ordered', label: 'stories ordered' },
  { key: 'review_closed', label: 'designer-review' },
  { key: 'dev_ok', label: 'dev-OK' }
]

export const NOTHING_DONE: IterationReadiness = {
  drafted: false,
  stories_ordered: false,
  review_closed: false,
  dev_ok: false
}

export const readinessOf = (iteration: BoardIterationData): IterationReadiness =>
  iteration.readiness ?? NOTHING_DONE

export const unmetCount = (readiness: IterationReadiness): number =>
  CHECK_ORDER.filter(({ key }) => !readiness[key]).length

export const nothingDone = (readiness: IterationReadiness): boolean =>
  CHECK_ORDER.every(({ key }) => !readiness[key])

export const firstUnmetLabel = (readiness: IterationReadiness): string | null =>
  CHECK_ORDER.find(({ key }) => !readiness[key])?.label ?? null
