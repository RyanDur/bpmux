import { test, expect } from '@playwright/test'
import { readFileSync } from 'fs'
import { join } from 'path'
import { SAFE_INHERITED_ENV_KEYS, pickSafeInheritedEnv } from './helpers/launch'

const LAUNCH_HELPER_PATH = join(__dirname, 'helpers', 'launch.ts')
const launchHelperSource = (): string => readFileSync(LAUNCH_HELPER_PATH, 'utf-8')

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
      BPMUX_REPO: 'acme/bpmux',
      BPMUX_GH_BASE_URL: 'https://api.github.com',
      BPMUX_DATA_BASE: '/tmp/leaked',
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
    expect(picked.BPMUX_DATA_BASE).toBeUndefined()
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

test.describe('#1205 launchIsolatedElectron wires pickSafeInheritedEnv at the env construction site', () => {
  test('launch.ts spreads pickSafeInheritedEnv(process.env), not process.env directly', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/\.\.\.pickSafeInheritedEnv\(process\.env\)/)
    expect(src).not.toMatch(/env:\s*\{\s*\.\.\.process\.env/)
  })

  test('launch.ts explicit env-overrides set BPMUX_REPO + GH_TOKEN to mock values, not inherited', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/ghEnvOverrides\['BPMUX_REPO'\]\s*=\s*'acme\/bpmux'/)
    expect(src).toMatch(/ghEnvOverrides\['GH_TOKEN'\]\s*=\s*'mock-gh-token'/)
  })
})

test.describe('#1204 launchIsolatedElectron isolates BPMUX_LOG to per-test tempdir', () => {
  test('launch.ts declares a defaultLogPath bound to a per-test mkdtemp dir', () => {
    const src = launchHelperSource()
    // #1291 refactored the mkdtempSync site so the path can be tracked
    // for ownedTempPaths cleanup. Gate asserts the substrate intent
    // (per-test mkdtemp + LOG-suffix join) rather than the exact
    // line-wrap, so the refactor doesn't break the structural check.
    expect(src).toMatch(/const\s+logDir\s*=\s*mkdtempSync\(/)
    expect(src).toMatch(/'bpmux-playwright-log-'/)
    expect(src).toMatch(/const\s+defaultLogPath\s*=\s*join\(\s*logDir\s*,\s*'bpmux-test-diag\.log'/)
  })

  test('launch.ts wires BPMUX_LOG into the spawned bpmux env from defaultLogPath', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/BPMUX_LOG:\s*defaultLogPath/)
  })

  test('isolation enumeration covers all three shared defaults together (auth-token, store, log)', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/BPMUX_AUTH_TOKEN_PATH:\s*defaultAuthTokenPath/)
    expect(src).toMatch(/BPMUX_BOARD_STORE_PATH:\s*defaultBoardStorePath/)
    expect(src).toMatch(/BPMUX_LOG:\s*defaultLogPath/)
  })
})
