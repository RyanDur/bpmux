import { type ApiContext, type HttpError, http } from '../api/http'
import { type AsyncResult } from '../lib/asyncResult'

export interface SpawnResponse {
  terminal_id?: string
  cwd?: string
}

export interface TerminalsListResponse {
  terminals?: { terminal_id: string; cwd?: string }[]
}

export const spawnTerminalRequest = (ctx: ApiContext): AsyncResult<SpawnResponse, HttpError> =>
  http.post<SpawnResponse>(ctx, '/api/terminals/spawn')

export const closeTerminalRequest = (
  ctx: ApiContext,
  id: string,
  force: boolean
): AsyncResult<void, HttpError> =>
  http.post(ctx, '/api/terminals/close', { terminal_id: id, force })

export const listTerminalsRequest = (
  ctx: ApiContext
): AsyncResult<TerminalsListResponse, HttpError> =>
  http.get<TerminalsListResponse>(ctx, '/api/terminals')
