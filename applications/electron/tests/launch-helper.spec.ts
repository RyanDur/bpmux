import { test, expect } from '@playwright/test'
import { SAFE_INHERITED_ENV_KEYS, pickSafeInheritedEnv } from './helpers/launch'

test.describe('#1205 launch-helper safe-inherited-env allow-list', () => {
  test('allow-list excludes every GH-routing env var bpmux tracker.rs reads', () => {
    const ghRoutingKeys = ['GH_TOKEN', 'GITHUB_TOKEN', 'BPMUX_REPO', 'BPMUX_GH_BASE_URL']
    for (const k of ghRoutingKeys) {
      expect(SAFE_INHERITED_ENV_KEYS).not.toContain(k)
    }
  })

  test('allow-list contains the system essentials Electron + bpmux need', () => {
    const required = ['PATH', 'HOME', 'TMPDIR', 'ELECTRON_RUN_AS_NODE']
    for (const k of required) {
      expect(SAFE_INHERITED_ENV_KEYS).toContain(k)
    }
  })

  test('pickSafeInheritedEnv copies only allow-listed keys + drops everything else', () => {
    const fakeEnv: NodeJS.ProcessEnv = {
      GH_TOKEN: 'real-secret-token',
      GITHUB_TOKEN: 'another-real-token',
      BPMUX_REPO: 'RyanDur/bpmux',
      BPMUX_GH_BASE_URL: 'https://api.github.com',
      BPMUX_SESSION_BASE: '/tmp/leaked',
      RANDOM_OTHER: 'leaked',
      PATH: '/usr/bin:/bin',
      HOME: '/Users/ryandurling',
      TMPDIR: '/tmp',
      ELECTRON_RUN_AS_NODE: '1'
    }
    const picked = pickSafeInheritedEnv(fakeEnv)

    expect(picked.GH_TOKEN).toBeUndefined()
    expect(picked.GITHUB_TOKEN).toBeUndefined()
    expect(picked.BPMUX_REPO).toBeUndefined()
    expect(picked.BPMUX_GH_BASE_URL).toBeUndefined()
    expect(picked.BPMUX_SESSION_BASE).toBeUndefined()
    expect(picked.RANDOM_OTHER).toBeUndefined()

    expect(picked.PATH).toBe('/usr/bin:/bin')
    expect(picked.HOME).toBe('/Users/ryandurling')
    expect(picked.TMPDIR).toBe('/tmp')
    expect(picked.ELECTRON_RUN_AS_NODE).toBe('1')
  })

  test('pickSafeInheritedEnv handles empty + undefined input', () => {
    const empty = pickSafeInheritedEnv({})
    expect(Object.keys(empty)).toHaveLength(0)

    const onlyGh = pickSafeInheritedEnv({
      GH_TOKEN: 'x',
      GITHUB_TOKEN: 'y',
      BPMUX_REPO: 'a/b',
      BPMUX_GH_BASE_URL: 'https://api.github.com'
    })
    expect(Object.keys(onlyGh)).toHaveLength(0)
  })
})
