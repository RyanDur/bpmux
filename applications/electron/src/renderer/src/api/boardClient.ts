/**
 * Board fetch client (#1352). Thin wrapper over `http.get` pinning
 * the endpoint + the typed BoardResponse shape. The optional
 * `signal` argument flows through to the underlying `fetch` so
 * callers can abort an in-flight refresh during navigation.
 */

import { type ApiContext, type HttpError, http } from './http'
import { type AsyncResult } from '../lib/asyncResult'
import { type BoardResponse } from '../board/types'

export const getBoard = (
  ctx: ApiContext,
  signal?: AbortSignal
): AsyncResult<BoardResponse, HttpError> => http.get<BoardResponse>(ctx, '/api/board', signal)
