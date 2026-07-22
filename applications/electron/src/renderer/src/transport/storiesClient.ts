import { type ApiContext, type HttpError, http } from './http'
import { type AsyncResult } from '../lib/asyncResult'

export interface AcceptPayload {
  would_file_bug: boolean
  force?: boolean
  bypass_reason?: string
}

export interface RejectPayload {
  note: string
}

export interface CommentPayload {
  body: string
}

export const postStoryComment = (
  ctx: ApiContext,
  storyId: number,
  payload: CommentPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/comments`, payload)

export const postStoryAccept = (
  ctx: ApiContext,
  storyId: number,
  payload: AcceptPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/accept`, payload)

export const postStoryReject = (
  ctx: ApiContext,
  storyId: number,
  payload: RejectPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/reject`, payload)
