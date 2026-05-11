import { _electron as electron, expect, type ElectronApplication, type Page } from '@playwright/test'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, symlinkSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import * as net from 'net'
import { startMockGhServer, type MockGhServer } from './mock_gh'

export const getFreePort = async (): Promise<number> =>
  new Promise((resolve, reject) => {
    const server = net.createServer()
    server.on('error', reject)
    server.listen(0, () => {
      const addr = server.address()
      if (typeof addr === 'object' && addr !== null) {
        const port = addr.port
        server.close(() => resolve(port))
      } else {
        reject(new Error('failed to bind ephemeral port'))
      }
    })
  })

export interface LaunchOpts {
  userDataDir?: string
  sessionBase?: string
  backendPort?: number
  seedMessages?: ReadonlyArray<Record<string, unknown>>
  /**
   * Pre-seed `<session-dir>/pins.json` so the Electron renderer's
   * `usePins` hook hydrates a non-empty pins state on mount (#489).
   * Map shape: channel key → ordered Pin records (matches
   * `session::pins::PinsStore` Rust shape on the wire).
   */
  seedPins?: Record<string, ReadonlyArray<Record<string, unknown>>>
  /**
   * Pre-seed `<session-dir>/roster_seed.json` so bpmux's in-memory
   * roster is pre-populated at startup with the listed identities
   * (e.g., `["dev:claude-code", "pm:claude-code"]`). Sister to
   * seedMessages — needed because bpmux's roster only populates via
   * live WS-connect; tests that seed-messages-only need this to
   * exercise sidebar's roster-derived peer entries (#1246).
   */
  seedRoster?: ReadonlyArray<string>
  extraEnv?: Record<string, string>
}

export interface LaunchResult {
  app: ElectronApplication
  sessionBase: string
  userDataDir: string
  backendPort: number
  mockGh?: MockGhServer
}

export const SAFE_INHERITED_ENV_KEYS = [
  'PATH',
  'HOME',
  'USER',
  'LOGNAME',
  'SHELL',
  'LANG',
  'LC_ALL',
  'LC_CTYPE',
  'LC_COLLATE',
  'TMPDIR',
  'TMP',
  'TEMP',
  'DISPLAY',
  'XAUTHORITY',
  'ELECTRON_RUN_AS_NODE',
  'NODE_PATH',
  'CI'
] as const

export const pickSafeInheritedEnv = (
  env: NodeJS.ProcessEnv = process.env
): NodeJS.ProcessEnv => {
  const picked: NodeJS.ProcessEnv = {}
  for (const k of SAFE_INHERITED_ENV_KEYS) {
    const v = env[k]
    if (v !== undefined) {
      picked[k] = v
    }
  }
  return picked
}

export const launchIsolatedElectron = async (opts: LaunchOpts = {}): Promise<LaunchResult> => {
  const sessionBase = opts.sessionBase ?? mkdtempSync(join(tmpdir(), 'bpmux-playwright-'))
  const userDataDir =
    opts.userDataDir ?? mkdtempSync(join(tmpdir(), 'bpmux-playwright-userdata-'))
  const backendPort = opts.backendPort ?? (await getFreePort())
  // #1290 Part B: roster_seed.json is loaded at Session::new from
  // session_dir/roster_seed.json (not via Storage trait — pre-#1290),
  // so seedRoster STAYS as a filesystem-write + 'current' symlink
  // setup (AC 10). seedMessages + seedPins moved to HTTP test-seed
  // API after waitForHealthz so they route through the active Storage
  // trait — works identically against FileStorage + MemoryStorage.
  const hasRoster = opts.seedRoster && opts.seedRoster.length > 0
  if (hasRoster) {
    const sessionId = 'playwright-seed-' + Date.now().toString(36)
    const sessionDir = join(sessionBase, sessionId)
    mkdirSync(sessionDir, { recursive: true })
    writeFileSync(
      join(sessionDir, 'roster_seed.json'),
      JSON.stringify(opts.seedRoster)
    )
    symlinkSync(sessionDir, join(sessionBase, 'current'))
  }
  // Per-test auth-token isolation. Without this, parallel workers
  // collide on $HOME/.bpmux/auth_token — bpmux's default location —
  // causing race-class flakes where one worker's token write reaches
  // another worker's reader. Each test gets its own tempdir-backed
  // path; callers that pass an explicit BPMUX_AUTH_TOKEN_PATH in
  // extraEnv override this default.
  const defaultAuthTokenPath = join(
    mkdtempSync(join(tmpdir(), 'bpmux-playwright-auth-')),
    'auth_token'
  )
  const defaultBoardStorePath = join(
    mkdtempSync(join(tmpdir(), 'bpmux-playwright-store-')),
    'board-store.json'
  )
  const defaultBoardOutboxPath = join(
    mkdtempSync(join(tmpdir(), 'bpmux-playwright-outbox-')),
    'board-outbox.json'
  )
  const defaultLogPath = join(
    mkdtempSync(join(tmpdir(), 'bpmux-playwright-log-')),
    'bpmux-test-diag.log'
  )

  const callerSetsGhBaseUrl = !!opts.extraEnv?.['BPMUX_GH_BASE_URL']
  let defaultMockGh: MockGhServer | undefined
  const ghEnvOverrides: Record<string, string> = {}
  if (!callerSetsGhBaseUrl) {
    defaultMockGh = await startMockGhServer()
    ghEnvOverrides['BPMUX_GH_BASE_URL'] = defaultMockGh.baseUrl
    ghEnvOverrides['BPMUX_REPO'] = 'acme/bpmux'
    ghEnvOverrides['GH_TOKEN'] = 'mock-gh-token'
  }

  const app = await electron.launch({
    args: ['out/main/index.js', `--user-data-dir=${userDataDir}`],
    env: {
      ...pickSafeInheritedEnv(process.env),
      BPMUX_SESSION_BASE: sessionBase,
      BPMUX_BACKEND_PORT: String(backendPort),
      BPMUX_AUTH_TOKEN_PATH: defaultAuthTokenPath,
      BPMUX_BOARD_STORE_PATH: defaultBoardStorePath,
      BPMUX_BOARD_OUTBOX_PATH: defaultBoardOutboxPath,
      BPMUX_LOG: defaultLogPath,
      // #1290 Part B: enable the test-seed endpoints
      // (/api/test/seed_messages, /api/test/seed_pins,
      // /api/test/seed_reactions). Structural gate — without this var
      // the routes are not mounted (production servers cannot reach
      // them).
      BPMUX_TEST_SEED_API: '1',
      // #1289 + #1290: route bpmux's session storage through
      // MemoryStorage so per-test backend-startup disk-I/O cost is
      // eliminated — the substrate-class that empirically falsified
      // macOS self-hosted e2e migration on f9ebe47/c8ba878/5bdb235.
      // Test fixtures still seed via the storage trait (post-Part A),
      // so seeds become observable identically to the FileStorage
      // path. Tests that need FileStorage explicitly (e.g., asserting
      // on-disk artifacts) can override via opts.extraEnv.
      BPMUX_STORAGE_BACKEND: 'memory',
      ...ghEnvOverrides,
      ...(opts.extraEnv ?? {})
    }
  })
  await waitForHealthz(backendPort)
  await seedMessagesAndPinsViaApi(backendPort, opts)
  return { app, sessionBase, userDataDir, backendPort, mockGh: defaultMockGh }
}

const seedMessagesAndPinsViaApi = async (
  backendPort: number,
  opts: LaunchOpts
): Promise<void> => {
  if (opts.seedMessages && opts.seedMessages.length > 0) {
    const res = await fetch(`http://127.0.0.1:${backendPort}/api/test/seed_messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ messages: opts.seedMessages })
    })
    if (!res.ok) {
      throw new Error(`seed_messages POST failed: ${res.status} ${await res.text()}`)
    }
  }
  if (opts.seedPins && Object.keys(opts.seedPins).length > 0) {
    const store = { channels: opts.seedPins }
    const res = await fetch(`http://127.0.0.1:${backendPort}/api/test/seed_pins`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ store })
    })
    if (!res.ok) {
      throw new Error(`seed_pins POST failed: ${res.status} ${await res.text()}`)
    }
  }
}

const waitForHealthz = async (port: number): Promise<void> => {
  const deadline = Date.now() + 60_000
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/healthz`)
      if (res.ok) return
    } catch {
      // backend not yet listening — retry
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
  }
  throw new Error(`backend on port ${port} did not become healthy within 60s`)
}

export const launchIsolatedWithWindow = async (
  opts: LaunchOpts = {}
): Promise<LaunchResult & { win: Page }> => {
  const result = await launchIsolatedElectron(opts)
  const win = await result.app.firstWindow()
  return { ...result, win }
}

// #1127: Playwright counterpart to #724's `TestBpmux::start_with_mock_gh`.
// Spawns an in-process httpmock-style GH server and points the spawned
// bpmux's GitHubSourceAdapter at it via BPMUX_GH_BASE_URL. Drops
// BPMUX_TEST_MODE entirely — server uses the prod orchestrator with
// the real adapter against the mock boundary. Tests drive mutations
// via /api/rpc/{tool} (auth-gated), not /api/test/* (deprecated as
// the InMemoryAdapter substrate retires).
export interface MockGhLaunchResult extends LaunchResult {
  win: Page
  mockGh: MockGhServer
  // Pre-read auth token so callers don't race the renderer's read.
  authToken: string
  // Bound `rpc(tool, body)` helper. Adds the bearer + agent-role +
  // host headers the gated /api/rpc/{tool} endpoint requires. Default
  // role 'dev'; pass `agent` in opts.callRole to override per-call.
  rpc: (tool: string, body: unknown, agent?: string) => Promise<Response>
}

export const launchWithMockGh = async (
  opts: LaunchOpts = {}
): Promise<MockGhLaunchResult> => {
  const mockGh = await startMockGhServer()
  // Per-test auth token — same isolation pattern as the default
  // launch helper, but we capture the path so we can read the
  // generated token after spawn.
  const authTokenPath = join(
    mkdtempSync(join(tmpdir(), 'bpmux-mockgh-auth-')),
    'auth_token'
  )
  const result = await launchIsolatedElectron({
    ...opts,
    extraEnv: {
      // Real GitHubSourceAdapter aimed at the in-process mock.
      // BPMUX_REPO + GH_TOKEN are required by `from_env()` even
      // though the mock ignores credentials.
      BPMUX_GH_BASE_URL: mockGh.baseUrl,
      BPMUX_REPO: 'acme/bpmux',
      GH_TOKEN: 'mock-gh-token',
      BPMUX_AUTH_TOKEN_PATH: authTokenPath,
      // Drain loop pushes pending BoardStore events (TagAdded /
      // TagRemoved / StoryClosed / etc.) to mock GH at this interval.
      // Production default is 5s; tests need fast turnaround so the
      // next gate's `refresh_story` sees the mutations the previous
      // gate enqueued.
      BPMUX_DRAIN_INTERVAL_SECS: '1',
      // Ingest poll re-fetches all stories from GH and seed_all-
      // replaces BoardStore. With drain at 1s, ingest at 1s would
      // race and clobber gate-handler-set state when ingest fires
      // BEFORE drain. Push ingest out of the test window (default
      // 60s, but pin explicitly to make the intent obvious).
      BPMUX_POLL_INTERVAL_SECS: '600',
      ...(opts.extraEnv ?? {})
    }
  })
  const win = await result.app.firstWindow()
  // Mock /api/roster as empty so the sidebar's useTeamRoster hook
  // falls back to message-derivation immediately. Without this,
  // useTeamRoster waits on the real bpmux backend's /api/roster
  // which races against message hydration under workers=2 contention.
  // Specs that need a populated roster can `win.route` over this
  // default.
  await win.route('**/api/roster', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ roster: [] })
    })
  })
  await win.reload()
  // Token file is written by the bpmux server on startup; healthz
  // already gates that. Read it once so RPC callers can authenticate.
  const authToken = readFileSync(authTokenPath, 'utf-8').trim()
  const rpc = async (tool: string, body: unknown, agent = 'dev'): Promise<Response> => {
    return fetch(`http://127.0.0.1:${result.backendPort}/api/rpc/${tool}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${authToken}`,
        'X-BPMUX-Agent': agent
      },
      body: JSON.stringify(body ?? {})
    })
  }
  return { ...result, win, mockGh, authToken, rpc }
}

export const waitForBoardLoaded = async (win: Page): Promise<void> => {
  await expect(win.locator('[data-test="board-region"]')).toBeVisible()
  await expect(win.locator('[data-test="board-list"]')).toBeVisible()
  await expect(win.locator('[data-test="board-mock-flag"]')).toHaveCount(0)
}

// `anchorText` gates downstream derived-state assertions on WS
// history actually arriving — message-list visibility alone races
// against hydration under workers=2 contention.
export const waitForMessagesLoaded = async (
  win: Page,
  anchorText?: string
): Promise<void> => {
  await expect(win.locator('[data-test="message-list"]')).toBeVisible()
  if (anchorText) {
    await expect(win.locator('[data-test="message-list"]')).toContainText(anchorText)
  }
}

