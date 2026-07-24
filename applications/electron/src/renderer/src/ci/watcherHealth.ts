export const WATCHER_UNHEALTHY_THRESHOLD_MS = 120_000

export type WatcherHealthInput = {
  last_successful_poll_at?: string | null
}

export const isWatcherUnhealthy = (
  state: WatcherHealthInput | null,
  now: number = Date.now()
): boolean => {
  if (!state || !state.last_successful_poll_at) return false
  const lastPollMs = Date.parse(state.last_successful_poll_at)
  if (Number.isNaN(lastPollMs)) return false
  return now - lastPollMs > WATCHER_UNHEALTHY_THRESHOLD_MS
}

export const watcherUnhealthyAriaLabel = (
  state: WatcherHealthInput | null,
  now: number = Date.now()
): string => {
  if (!state?.last_successful_poll_at) {
    return 'CI watcher unhealthy — no successful poll observed'
  }
  const ageSecs = Math.round((now - Date.parse(state.last_successful_poll_at)) / 1000)
  return `CI watcher unhealthy — last successful poll ${ageSecs}s ago`
}

export const derivedClassName = (conclusion: string, watcherUnhealthy: boolean): string =>
  `ci-glyph ci-glyph--${conclusion}${watcherUnhealthy ? ' ci-glyph--watcher-unhealthy' : ''}`

export const derivedDataState = (conclusion: string, watcherUnhealthy: boolean): string =>
  watcherUnhealthy ? 'watcher-unhealthy' : conclusion
