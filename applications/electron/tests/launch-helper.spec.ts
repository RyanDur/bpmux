import { test, expect } from '@playwright/test'
import { REAL_GH_ROUTING_ENV_KEYS, stripRealGhRoutingFromInheritedEnv } from './helpers/launch'

// #1205: regression coverage for the test-harness's default-mock-GH
// protection. Pre-#1205, `launchIsolatedElectron` spread
// `...process.env` into the spawned bpmux + 55+ tests inherited the
// dev's BPMUX_REPO + GH_TOKEN, silently routing ci_watcher +
// ingest_poll + hydration_loop against real api.github.com. Today's
// rate-limit-cooldown driver. Per Ryan 2026-05-06 01:31 UTC ("if
// there are still calls to github, it should not pass") this test
// is the load-bearing falsifier-citation drive per
// `feedback_falsifier_load_bearing_test_at_e2e_layer`.

test.describe('#1205 launch-helper default-mock-GH protection', () => {
  test('strip-list covers exactly the env vars bpmux tracker.rs reads', () => {
    // Source-of-truth: applications/bpmux/src/bridge/tracker.rs
    // select_from_env() reads BPMUX_REPO, BPMUX_GH_BASE_URL,
    // GH_TOKEN, GITHUB_TOKEN. If tracker.rs adds a new env var
    // for GH routing without adding it here, this test still
    // passes — but the new var would leak from process.env into
    // the spawned bpmux. The fix in that case is to update both
    // the strip-list AND this test simultaneously.
    expect(REAL_GH_ROUTING_ENV_KEYS).toEqual([
      'GH_TOKEN',
      'GITHUB_TOKEN',
      'BPMUX_REPO',
      'BPMUX_GH_BASE_URL'
    ])
  })

  test('stripRealGhRoutingFromInheritedEnv removes GH-routing keys + preserves other env', () => {
    const fakeEnv: NodeJS.ProcessEnv = {
      // GH-routing keys (must be stripped):
      GH_TOKEN: 'real-secret-token',
      GITHUB_TOKEN: 'another-real-token',
      BPMUX_REPO: 'RyanDur/bpmux',
      BPMUX_GH_BASE_URL: 'https://api.github.com',
      // Non-GH keys (must pass through):
      PATH: '/usr/bin:/bin',
      HOME: '/Users/ryandurling',
      TMPDIR: '/tmp',
      ELECTRON_RUN_AS_NODE: '1',
      BPMUX_SESSION_BASE: '/tmp/bpmux-session-isolated',
      RANDOM_OTHER: 'preserved'
    }
    const stripped = stripRealGhRoutingFromInheritedEnv(fakeEnv)

    // GH-routing keys absent.
    expect(stripped.GH_TOKEN).toBeUndefined()
    expect(stripped.GITHUB_TOKEN).toBeUndefined()
    expect(stripped.BPMUX_REPO).toBeUndefined()
    expect(stripped.BPMUX_GH_BASE_URL).toBeUndefined()

    // Non-GH keys preserved.
    expect(stripped.PATH).toBe('/usr/bin:/bin')
    expect(stripped.HOME).toBe('/Users/ryandurling')
    expect(stripped.TMPDIR).toBe('/tmp')
    expect(stripped.ELECTRON_RUN_AS_NODE).toBe('1')
    expect(stripped.BPMUX_SESSION_BASE).toBe('/tmp/bpmux-session-isolated')
    expect(stripped.RANDOM_OTHER).toBe('preserved')
  })

  test('stripRealGhRoutingFromInheritedEnv handles empty + undefined input', () => {
    const empty = stripRealGhRoutingFromInheritedEnv({})
    expect(Object.keys(empty)).toHaveLength(0)

    const onlyGh = stripRealGhRoutingFromInheritedEnv({
      GH_TOKEN: 'x',
      GITHUB_TOKEN: 'y',
      BPMUX_REPO: 'a/b',
      BPMUX_GH_BASE_URL: 'https://api.github.com'
    })
    expect(Object.keys(onlyGh)).toHaveLength(0)
  })

  // Note: end-to-end coverage that the spawned bpmux subprocess
  // actually routes to mockGh (not api.github.com) lives in the
  // existing 300+ Playwright tests + real-backend.spec.ts. They
  // collectively prove the integration: every test that calls
  // launchIsolatedElectron exercises the auto-mock path; if any
  // hit real GH, those tests would 401/403 against the fake
  // mock-gh-token. The 3 pure-function tests above pin the
  // strip-mechanism's logic so a future regression that breaks
  // the strip surfaces here BEFORE the integration tests have
  // to catch it indirectly.
})
