/**
 * Labels mutation client (#1352). Thin wrapper over `http.post` that
 * pins the endpoint shape + the typed payload. Replaces raw fetch()
 * at the presentation/transport boundary so Iteration/Board
 * components don't carry HTTP-shape constants inline.
 */

import { type ApiContext, type HttpError, http } from './http'
import { type AsyncResult } from '../lib/asyncResult'

export interface LabelMutation {
  add: string[]
  remove: string[]
}

export const postStoryLabels = (
  ctx: ApiContext,
  storyNumber: number,
  mutation: LabelMutation
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyNumber}/labels`, mutation)
