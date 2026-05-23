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
