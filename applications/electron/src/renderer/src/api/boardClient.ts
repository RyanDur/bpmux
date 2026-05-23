import { type ApiContext, type HttpError, http } from './http'
import { type AsyncResult } from '../lib/asyncResult'
import { type BoardResponse } from '../board/types'

export const getBoard = (
  ctx: ApiContext,
  signal?: AbortSignal
): AsyncResult<BoardResponse, HttpError> => http.get<BoardResponse>(ctx, '/api/board', signal)
