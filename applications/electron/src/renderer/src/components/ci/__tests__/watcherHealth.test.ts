import { describe, expect, it } from 'vitest'
import {
  WATCHER_UNHEALTHY_THRESHOLD_MS,
  derivedClassName,
  derivedDataState,
  isWatcherUnhealthy,
  watcherUnhealthyAriaLabel
} from '../watcherHealth'

const NOW = Date.parse('2026-05-05T00:00:00.000Z')

const isoSecondsAgo = (secs: number): string => new Date(NOW - secs * 1000).toISOString()

describe('watcherHealth pure helpers', () => {
  describe('isWatcherUnhealthy', () => {
    it('returns false when state is null', () => {
      expect(isWatcherUnhealthy(null, NOW)).toBe(false)
    })

    it('returns false when last_successful_poll_at is null', () => {
      expect(isWatcherUnhealthy({ last_successful_poll_at: null }, NOW)).toBe(false)
    })

    it('returns false when last_successful_poll_at is undefined', () => {
      expect(isWatcherUnhealthy({}, NOW)).toBe(false)
    })

    it('returns false when last_successful_poll_at is invalid RFC3339', () => {
      expect(isWatcherUnhealthy({ last_successful_poll_at: 'not-a-date' }, NOW)).toBe(false)
    })

    it('returns false when poll-age is below threshold', () => {
      const ageSecs = WATCHER_UNHEALTHY_THRESHOLD_MS / 1000 - 5
      expect(isWatcherUnhealthy({ last_successful_poll_at: isoSecondsAgo(ageSecs) }, NOW)).toBe(
        false
      )
    })

    it('returns true when poll-age exceeds threshold', () => {
      const ageSecs = WATCHER_UNHEALTHY_THRESHOLD_MS / 1000 + 5
      expect(isWatcherUnhealthy({ last_successful_poll_at: isoSecondsAgo(ageSecs) }, NOW)).toBe(
        true
      )
    })

    it('returns false at exactly threshold (strictly greater)', () => {
      const ageSecs = WATCHER_UNHEALTHY_THRESHOLD_MS / 1000
      expect(isWatcherUnhealthy({ last_successful_poll_at: isoSecondsAgo(ageSecs) }, NOW)).toBe(
        false
      )
    })
  })

  describe('watcherUnhealthyAriaLabel', () => {
    it('returns no-successful-poll-observed message when timestamp absent', () => {
      expect(watcherUnhealthyAriaLabel(null, NOW)).toContain('no successful poll observed')
      expect(watcherUnhealthyAriaLabel({}, NOW)).toContain('no successful poll observed')
    })

    it('includes computed age in seconds when timestamp present', () => {
      const ageSecs = 150
      const label = watcherUnhealthyAriaLabel(
        { last_successful_poll_at: isoSecondsAgo(ageSecs) },
        NOW
      )
      expect(label).toContain('last successful poll 150s ago')
    })

    it('rounds age to nearest second', () => {
      const lastPoll = new Date(NOW - 150_700).toISOString()
      const label = watcherUnhealthyAriaLabel({ last_successful_poll_at: lastPoll }, NOW)
      expect(label).toMatch(/\b15[01]s ago\b/)
    })
  })

  describe('derivedClassName', () => {
    it('returns base + conclusion class when watcher healthy', () => {
      expect(derivedClassName('success', false)).toBe('ci-glyph ci-glyph--success')
    })

    it('appends watcher-unhealthy class when watcher unhealthy', () => {
      expect(derivedClassName('success', true)).toBe(
        'ci-glyph ci-glyph--success ci-glyph--watcher-unhealthy'
      )
    })

    it('handles unknown conclusion + unhealthy combo', () => {
      expect(derivedClassName('unknown', true)).toBe(
        'ci-glyph ci-glyph--unknown ci-glyph--watcher-unhealthy'
      )
    })
  })

  describe('derivedDataState', () => {
    it('returns conclusion when watcher healthy', () => {
      expect(derivedDataState('success', false)).toBe('success')
      expect(derivedDataState('failure', false)).toBe('failure')
    })

    it('returns watcher-unhealthy when watcher unhealthy regardless of conclusion', () => {
      expect(derivedDataState('success', true)).toBe('watcher-unhealthy')
      expect(derivedDataState('failure', true)).toBe('watcher-unhealthy')
      expect(derivedDataState('unknown', true)).toBe('watcher-unhealthy')
    })
  })
})
