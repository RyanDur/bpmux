import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { existsSync, statSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const execFileAsync = promisify(execFile)

export const builtBpmuxBinary = (): string => {
  const workspace = join(__dirname, '../../../..')
  const release = join(workspace, 'target/release/bpmux')
  const debug = join(workspace, 'target/debug/bpmux')
  const releaseExists = existsSync(release)
  const debugExists = existsSync(debug)
  if (releaseExists && debugExists) {
    return statSync(release).mtimeMs >= statSync(debug).mtimeMs ? release : debug
  }
  return releaseExists ? release : debug
}

export interface BpmuxCli {
  run: (role: string, args: string[]) => Promise<string>
  numberFrom: (stdout: string) => number
  cleanup: () => void
}

export const makeBpmuxCli = (backendPort: number, authToken: string): BpmuxCli => {
  const authDir = mkdtempSync(join(tmpdir(), 'bpmux-cli-journey-auth-'))
  const tokenPath = join(authDir, 'auth_token')
  writeFileSync(tokenPath, authToken)

  const run = async (role: string, args: string[]): Promise<string> => {
    const { stdout } = await execFileAsync(builtBpmuxBinary(), args, {
      env: {
        ...process.env,
        BPMUX_PORT: String(backendPort),
        BPMUX_ROLE: role,
        BPMUX_AUTH_TOKEN_PATH: tokenPath
      }
    })
    return stdout
  }

  const numberFrom = (stdout: string): number => Number(stdout.match(/#(\d+)/)?.[1])

  const cleanup = (): void => {
    try {
      rmSync(authDir, { recursive: true, force: true })
    } catch {
      void 0
    }
  }

  return { run, numberFrom, cleanup }
}
