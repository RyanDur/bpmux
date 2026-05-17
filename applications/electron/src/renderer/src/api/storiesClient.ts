/**
 * Stories mutation client (#1352). Replaces raw fetch() at the
 * presentation/transport boundary for the three DrillIn-driven
 * owner-gate routes: comment append, accept, reject.
 *
 * Sister to labelsClient.ts — same tagged-result shape, same
 * ApiContext / ClientError types. Re-exports those so callers can
 * import the full client surface from one module path if they
 * touch multiple stories endpoints.
 */

import type { ApiContext, ClientError, ClientResult } from './labelsClient'

export type { ApiContext, ClientError, ClientResult }

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

async function postStoryEndpoint<P>(
  ctx: ApiContext,
  storyNumber: number,
  endpoint: 'comments' | 'accept' | 'reject',
  payload: P,
  errorDetail: 'text' | 'none'
): Promise<ClientResult> {
  try {
    const res = await fetch(`${ctx.apiBase}/api/stories/${storyNumber}/${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${ctx.authToken ?? ''}`
      },
      body: JSON.stringify(payload)
    })
    if (!res.ok) {
      const detail =
        errorDetail === 'text' ? await res.text().catch(() => '') : ''
      const message = detail ? `HTTP ${res.status} — ${detail}` : `HTTP ${res.status}`
      return {
        ok: false,
        error: { kind: 'http', status: res.status, message }
      }
    }
    return { ok: true, value: undefined }
  } catch (err) {
    return {
      ok: false,
      error: {
        kind: 'network',
        message: err instanceof Error ? err.message : String(err)
      }
    }
  }
}

export function postStoryComment(
  ctx: ApiContext,
  storyNumber: number,
  payload: CommentPayload
): Promise<ClientResult> {
  return postStoryEndpoint(ctx, storyNumber, 'comments', payload, 'none')
}

export function postStoryAccept(
  ctx: ApiContext,
  storyNumber: number,
  payload: AcceptPayload
): Promise<ClientResult> {
  return postStoryEndpoint(ctx, storyNumber, 'accept', payload, 'text')
}

export function postStoryReject(
  ctx: ApiContext,
  storyNumber: number,
  payload: RejectPayload
): Promise<ClientResult> {
  return postStoryEndpoint(ctx, storyNumber, 'reject', payload, 'text')
}
