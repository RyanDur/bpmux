import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync } from 'node:fs'
import net from 'node:net'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'

interface VitestProvideContext {
  provide: <K extends keyof ProvidedTestContext>(key: K, value: ProvidedTestContext[K]) => void
}

interface ProvidedTestContext {
  bpmuxBackendPort: number
  bpmuxAuthToken: string
}

const REPO_ROOT_RELATIVE = join('..', '..', '..', '..')
const BACKEND_READY_TIMEOUT_MS = 15_000
const TOKEN_FILE_TIMEOUT_MS = 5_000
const TEARDOWN_TIMEOUT_MS = 3_000

let backendProc: ChildProcess | null = null
let tempSessionDir: string | null = null

function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = net.createServer()
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address()
      if (typeof addr === 'object' && addr !== null) {
        const port = addr.port
        server.close(() => resolve(port))
      } else {
        reject(new Error('getFreePort: failed to read address'))
      }
    })
    server.on('error', reject)
  })
}

async function waitForBackendReady(port: number): Promise<void> {
  const deadline = Date.now() + BACKEND_READY_TIMEOUT_MS
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`http://127.0.0.1:${port}/api/board`)
      if (res.status === 200 || res.status === 401) {
        return
      }
    } catch {}
    await sleep(100)
  }
  throw new Error(
    `bpmux backend on port ${port} did not respond within ${BACKEND_READY_TIMEOUT_MS}ms`
  )
}

async function waitForAuthToken(tokenPath: string): Promise<string> {
  const deadline = Date.now() + TOKEN_FILE_TIMEOUT_MS
  while (Date.now() < deadline) {
    if (existsSync(tokenPath)) {
      const contents = readFileSync(tokenPath, 'utf8').trim()
      if (contents.length > 0) {
        return contents
      }
    }
    await sleep(50)
  }
  throw new Error(`auth token file not populated at ${tokenPath}`)
}

export default async function setup({
  provide
}: VitestProvideContext): Promise<() => Promise<void>> {
  const thisFile = fileURLToPath(import.meta.url)
  const repoRoot = join(dirname(thisFile), REPO_ROOT_RELATIVE)
  const bpmuxBinary = join(repoRoot, 'target', 'debug', 'bpmux')
  if (!existsSync(bpmuxBinary)) {
    throw new Error(
      `bpmux binary not found at ${bpmuxBinary}. Run 'cargo build -p bpmux' before vitest.`
    )
  }
  tempSessionDir = mkdtempSync(join(tmpdir(), 'bpmux-rtl-'))
  const port = await getFreePort()
  const authTokenPath = join(tempSessionDir, 'auth_token')

  backendProc = spawn(
    bpmuxBinary,
    [
      '--name',
      'test',
      '--mcp-port',
      String(port),
      '--session-base',
      tempSessionDir,
      'server'
    ],
    {
      env: {
        ...process.env,
        BPMUX_TEST_MODE: '1',
        BPMUX_TEST_SEED_API: '1',
        BPMUX_STORAGE_BACKEND: 'memory',
        BPMUX_PRIORITY_PATH: join(tempSessionDir, 'priority.json'),
        BPMUX_USAGE_PATH: join(tempSessionDir, 'usage.json'),
        BPMUX_AUTH_TOKEN_PATH: authTokenPath,
        BPMUX_BOARD_STORE_PATH: join(tempSessionDir, 'board-store.json'),
        BPMUX_BOARD_OUTBOX_PATH: join(tempSessionDir, 'board-outbox.json'),
        BPMUX_SESSIONS_BASE: tempSessionDir
      },
      stdio: ['ignore', 'pipe', 'pipe']
    }
  )
  backendProc.stderr?.on('data', (chunk: Buffer) => {
    process.stderr.write(`[bpmux-test-backend] ${chunk.toString()}`)
  })
  backendProc.on('error', (err) => {
    console.error('bpmux test backend spawn error:', err)
  })

  await waitForBackendReady(port)
  const authToken = await waitForAuthToken(authTokenPath)

  provide('bpmuxBackendPort', port)
  provide('bpmuxAuthToken', authToken)

  return async () => {
    const proc = backendProc
    if (proc === null) return
    proc.kill('SIGTERM')
    await new Promise<void>((resolve) => {
      if (proc.exitCode !== null) {
        resolve()
        return
      }
      const timer = setTimeout(() => resolve(), TEARDOWN_TIMEOUT_MS)
      proc.once('exit', () => {
        clearTimeout(timer)
        resolve()
      })
    })
    backendProc = null
  }
}
