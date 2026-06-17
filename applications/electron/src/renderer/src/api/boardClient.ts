import { type ApiContext, type HttpError, http } from './http'
import { type AsyncResult } from '../lib/asyncResult'
import { type BoardResponse } from '../board/types'
import { boardResponseDecoder } from '../board/boardDecoder'
import { validate } from './validate'

export const getBoard = (
  ctx: ApiContext,
  signal?: AbortSignal
): AsyncResult<BoardResponse, HttpError> =>
  http.get<unknown>(ctx, '/api/board', signal).andThen(validate(boardResponseDecoder))
