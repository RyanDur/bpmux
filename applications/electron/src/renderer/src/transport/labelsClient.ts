import { type ApiContext, type HttpError, http } from './http'
import { type Result } from '@ryandur/sand'

export interface LabelMutation {
  add: string[]
  remove: string[]
}

export const postStoryLabels = (
  ctx: ApiContext,
  storyId: number,
  mutation: LabelMutation
): Result.Async<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/labels`, mutation)
