import { test, expect } from '@playwright/test'
import { REAL_GH_ROUTING_ENV_KEYS, stripRealGhRoutingFromInheritedEnv } from './helpers/launch'

test.describe('#1205 launch-helper default-mock-GH protection', () => {
  test('strip-list covers exactly the env vars bpmux tracker.rs reads', () => {
    expect(REAL_GH_ROUTING_ENV_KEYS).toEqual([
      'GH_TOKEN',
      'GITHUB_TOKEN',
      'BPMUX_REPO',
      'BPMUX_GH_BASE_URL'
    ])
  })

  test('stripRealGhRoutingFromInheritedEnv removes GH-routing keys + preserves other env', () => {
    const fakeEnv: NodeJS.ProcessEnv = {
      GH_TOKEN: 'real-secret-token',
      GITHUB_TOKEN: 'another-real-token',
      BPMUX_REPO: 'RyanDur/bpmux',
      BPMUX_GH_BASE_URL: 'https://api.github.com',
      PATH: '/usr/bin:/bin',
      HOME: '/Users/ryandurling',
      TMPDIR: '/tmp',
      ELECTRON_RUN_AS_NODE: '1',
      BPMUX_SESSION_BASE: '/tmp/bpmux-session-isolated',
      RANDOM_OTHER: 'preserved'
    }
    const stripped = stripRealGhRoutingFromInheritedEnv(fakeEnv)

    expect(stripped.GH_TOKEN).toBeUndefined()
    expect(stripped.GITHUB_TOKEN).toBeUndefined()
    expect(stripped.BPMUX_REPO).toBeUndefined()
    expect(stripped.BPMUX_GH_BASE_URL).toBeUndefined()

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
})
