import { type Maybe, maybe, not } from '@ryandur/sand'

export const WATCHER_UNHEALTHY_THRESHOLD_MS = 120_000

export type WatcherHealthInput = {
  last_successful_poll_at?: string | null
}

const lastPollOf = (state: Maybe<WatcherHealthInput>): Maybe<number> =>
  state
    .mBind((input) => maybe(input.last_successful_poll_at))
    .map(Date.parse)
    .mBind((lastPollMs) => maybe(lastPollMs, (parsed) => not(Number.isNaN(parsed))))

export const isWatcherUnhealthy = (
  state: Maybe<WatcherHealthInput>,
  now: number = Date.now()
): boolean =>
  lastPollOf(state)
    .map((lastPollMs) => now - lastPollMs > WATCHER_UNHEALTHY_THRESHOLD_MS)
    .orElse(false)

export const watcherUnhealthyAriaLabel = (
  state: Maybe<WatcherHealthInput>,
  now: number = Date.now()
): string =>
  lastPollOf(state)
    .map((lastPollMs) => Math.round((now - lastPollMs) / 1000))
    .map((ageSecs) => `CI watcher unhealthy — last successful poll ${ageSecs}s ago`)
    .orElse('CI watcher unhealthy — no successful poll observed')

export const derivedClassName = (conclusion: string, watcherUnhealthy: boolean): string =>
  `ci-glyph ci-glyph--${conclusion}${watcherUnhealthy ? ' ci-glyph--watcher-unhealthy' : ''}`

export const derivedDataState = (conclusion: string, watcherUnhealthy: boolean): string =>
  watcherUnhealthy ? 'watcher-unhealthy' : conclusion
