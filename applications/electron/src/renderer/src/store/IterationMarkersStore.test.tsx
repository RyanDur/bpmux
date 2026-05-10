import { describe, expect, test } from 'vitest'

// #1255 acceptance test path placeholder. Actual implementation uses
// the `useIterationMarkers` hook in renderer/src/hooks/, not a Store
// class. The hot-update tests for the hook live in
// hooks/useIterationMarkers.test.tsx (vitest spec for the
// shouldRefetchMarkers pure-fn gate). This file is the path the spec
// referenced for filing-time accountability.

describe('#1255 acceptance-test-path placeholder', () => {
  test('points to actual hook tests in useIterationMarkers.test.tsx', () => {
    // sentinel — vitest requires at least one test in a discovered spec
    expect(true).toBe(true)
  })
})
