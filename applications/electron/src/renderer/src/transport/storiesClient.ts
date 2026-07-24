import { type ApiContext, type HttpError, http } from './http'
import { type Result } from '@ryandur/sand'

export type AcceptPayload = {
  would_file_bug: boolean
  force?: boolean
  bypass_reason?: string
}

export type RejectPayload = {
  note: string
}

export type CommentPayload = {
  body: string
}

export const postStoryComment = (
  ctx: ApiContext,
  storyId: number,
  payload: CommentPayload
): Result.Async<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/comments`, payload)

export const postStoryAccept = (
  ctx: ApiContext,
  storyId: number,
  payload: AcceptPayload
): Result.Async<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/accept`, payload)

export const postStoryReject = (
  ctx: ApiContext,
  storyId: number,
  payload: RejectPayload
): Result.Async<void, HttpError> => http.post(ctx, `/api/stories/${storyId}/reject`, payload)
