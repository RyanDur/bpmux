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

export interface Comment {
  id: number
  author: string
  body: string
  created_at: string
}

export const getStoryComments = (
  ctx: ApiContext,
  storyNumber: number,
  signal?: AbortSignal
): AsyncResult<Comment[], HttpError> =>
  http.get<Comment[]>(ctx, `/api/stories/${storyNumber}/comments`, signal)

export const postStoryComment = (
  ctx: ApiContext,
  storyNumber: number,
  payload: CommentPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyNumber}/comments`, payload)

export const postStoryAccept = (
  ctx: ApiContext,
  storyNumber: number,
  payload: AcceptPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyNumber}/accept`, payload)

export const postStoryReject = (
  ctx: ApiContext,
  storyNumber: number,
  payload: RejectPayload
): AsyncResult<void, HttpError> => http.post(ctx, `/api/stories/${storyNumber}/reject`, payload)
