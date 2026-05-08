import { test, expect } from '@playwright/test'
import { readFileSync } from 'fs'
import { join } from 'path'
import {
  SAFE_INHERITED_ENV_KEYS,
  pickSafeInheritedEnv,
  launchIsolatedElectron
} from './helpers/launch'

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

test.describe('#1205 r4 e2e falsifier-(d) — shell GH-routing env does not reach spawned bpmux', () => {
  test('process.env.BPMUX_REPO + GH_TOKEN canary values do not appear in mockGh request log', async () => {
    test.setTimeout(90_000)
    const CANARY_REPO = 'CANARY-LEAKED-REPO/should-not-appear'
    const CANARY_TOKEN = 'CANARY-LEAKED-TOKEN-should-not-appear'

    const priorRepo = process.env.BPMUX_REPO
    const priorToken = process.env.GH_TOKEN
    const priorGithubToken = process.env.GITHUB_TOKEN
    const priorBaseUrl = process.env.BPMUX_GH_BASE_URL
    process.env.BPMUX_REPO = CANARY_REPO
    process.env.GH_TOKEN = CANARY_TOKEN
    process.env.GITHUB_TOKEN = CANARY_TOKEN
    process.env.BPMUX_GH_BASE_URL = 'https://api.github.com'

    let app: import('@playwright/test').ElectronApplication | undefined
    try {
      const result = await launchIsolatedElectron({
        extraEnv: { BPMUX_POLL_INTERVAL_SECS: '600' }
      })
      app = result.app
      const mockGh = result.mockGh
      expect(mockGh).toBeDefined()

      await expect
        .poll(() => mockGh!.requestLog().length, { timeout: 15_000 })
        .toBeGreaterThan(0)

      const log = mockGh!.requestLog()
      for (const req of log) {
        expect(req.path).not.toContain('CANARY-LEAKED-REPO')
        expect(req.authorization ?? '').not.toContain('CANARY-LEAKED-TOKEN')
      }

      const acmePathCount = log.filter((r) => r.path.includes('/repos/acme/bpmux/')).length
      expect(acmePathCount).toBeGreaterThan(0)
      expect(log.length).toBe(acmePathCount)
    } finally {
      if (app) await app.close()
      restoreEnv('BPMUX_REPO', priorRepo)
      restoreEnv('GH_TOKEN', priorToken)
      restoreEnv('GITHUB_TOKEN', priorGithubToken)
      restoreEnv('BPMUX_GH_BASE_URL', priorBaseUrl)
    }
  })
})

const restoreEnv = (key: string, prior: string | undefined): void => {
  if (prior === undefined) {
    delete process.env[key]
  } else {
    process.env[key] = prior
  }
}

test.describe('#1204 launchIsolatedElectron isolates BPMUX_LOG to per-test tempdir', () => {
  test('launch.ts declares a defaultLogPath bound to a per-test mkdtemp dir', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/const\s+defaultLogPath\s*=\s*join\(\s*mkdtempSync\(/)
    expect(src).toMatch(/'bpmux-playwright-log-'/)
    expect(src).toMatch(/'bpmux-test-diag\.log'/)
  })

  test('launch.ts wires BPMUX_LOG into the spawned bpmux env from defaultLogPath', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/BPMUX_LOG:\s*defaultLogPath/)
  })

  test('isolation enumeration covers all four shared defaults together (auth-token, store, outbox, log)', () => {
    const src = launchHelperSource()
    expect(src).toMatch(/BPMUX_AUTH_TOKEN_PATH:\s*defaultAuthTokenPath/)
    expect(src).toMatch(/BPMUX_BOARD_STORE_PATH:\s*defaultBoardStorePath/)
    expect(src).toMatch(/BPMUX_BOARD_OUTBOX_PATH:\s*defaultBoardOutboxPath/)
    expect(src).toMatch(/BPMUX_LOG:\s*defaultLogPath/)
  })
})
