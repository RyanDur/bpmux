import { type ApiContext, type HttpError, http } from '../transport/http'
import { type Result } from '@ryandur/sand'

export type SpawnResponse = {
  terminal_id?: string
  cwd?: string
}

export type TerminalsListResponse = {
  terminals?: { terminal_id: string; cwd?: string }[]
}

export const spawnTerminalRequest = (ctx: ApiContext): Result.Async<SpawnResponse, HttpError> =>
  http.post<SpawnResponse>(ctx, '/api/terminals/spawn')

export const closeTerminalRequest = (
  ctx: ApiContext,
  id: string,
  force: boolean
): Result.Async<void, HttpError> =>
  http.post(ctx, '/api/terminals/close', { terminal_id: id, force })

export const listTerminalsRequest = (
  ctx: ApiContext
): Result.Async<TerminalsListResponse, HttpError> =>
  http.get<TerminalsListResponse>(ctx, '/api/terminals')
