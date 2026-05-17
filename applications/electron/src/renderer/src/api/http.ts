/**
 * Generic HTTP verb module (#1352 / Ryan-pattern). Wraps the global
 * `fetch` in `AsyncResult<T, HttpError>` so callers branch on
 * `result.kind === 'success'` instead of try-catch + manual
 * res.ok checks. Sister-shape to the battleship repo's
 * `protocols/http/http.ts`.
 *
 * Verbs (`get`, `post`, `put`, `delete`) each bind:
 *  - The HTTP method.
 *  - The success-status allowlist (per-verb so a POST returning 204
 *    isn't a server error and a GET returning 201 is a server error).
 *  - The Accept / Content-Type / Authorization headers.
 *  - JSON decode + tagged-failure on network OR non-allowlist status.
 *
 * `ApiContext` carries the per-call apiBase + authToken; production
 * callers pass `window.bpmux` fields; tests inject a fake. The
 * generic `T` parameter is the success-body shape; pass `void` (or
 * leave defaulted) when the response has no useful body.
 */

import { type AsyncResult, asyncFailure, ofPromise } from '../lib/asyncResult'
import { failure, success, type Result } from '../lib/result'

export interface ApiContext {
  apiBase: string
  authToken: string | null | undefined
}

export type HttpError =
  | { kind: 'http'; status: number; body: string }
  | { kind: 'network'; message: string }
  | { kind: 'decode'; message: string }

const STATUS_OK = [200]
const STATUS_OK_NO_CONTENT = [200, 201, 204]

const buildHeaders = (
  ctx: ApiContext,
  includeContentType: boolean
): Record<string, string> => {
  const headers: Record<string, string> = {
    Accept: 'application/json',
    Authorization: `Bearer ${ctx.authToken ?? ''}`
  }
  if (includeContentType) {
    headers['Content-Type'] = 'application/json'
  }
  return headers
}

const request = async <T>(
  ctx: ApiContext,
  endpoint: string,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
  body: unknown,
  successStatuses: number[],
  signal?: AbortSignal
): Promise<Result<T, HttpError>> => {
  const url = `${ctx.apiBase}${endpoint}`
  const init: RequestInit = {
    method,
    headers: buildHeaders(ctx, body !== undefined)
  }
  if (body !== undefined) {
    init.body = JSON.stringify(body)
  }
  if (signal) {
    init.signal = signal
  }
  let res: Response
  try {
    res = await fetch(url, init)
  } catch (err) {
    return failure<HttpError, T>({
      kind: 'network',
      message: err instanceof Error ? err.message : String(err)
    })
  }
  if (!successStatuses.includes(res.status)) {
    const bodyText = await res.text().catch(() => '')
    return failure<HttpError, T>({
      kind: 'http',
      status: res.status,
      body: bodyText
    })
  }
  if (res.status === 204) {
    return success<T, HttpError>(undefined as unknown as T)
  }
  try {
    const json = (await res.json()) as T
    return success<T, HttpError>(json)
  } catch (err) {
    return failure<HttpError, T>({
      kind: 'decode',
      message: err instanceof Error ? err.message : String(err)
    })
  }
}

export const http = {
  get: <T>(
    ctx: ApiContext,
    endpoint: string,
    signal?: AbortSignal
  ): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'GET', undefined, STATUS_OK, signal)),

  post: <T = void>(
    ctx: ApiContext,
    endpoint: string,
    body?: unknown
  ): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'POST', body, STATUS_OK_NO_CONTENT)),

  put: <T = void>(
    ctx: ApiContext,
    endpoint: string,
    body?: unknown
  ): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'PUT', body, STATUS_OK_NO_CONTENT)),

  delete: <T = void>(
    ctx: ApiContext,
    endpoint: string,
    body?: unknown
  ): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'DELETE', body, STATUS_OK_NO_CONTENT))
}

/**
 * Format an `HttpError` as a single human-readable string for toast/
 * inline-error display. Callers that need to branch on the error
 * shape match on `error.kind` directly instead.
 */
export const describeHttpError = (error: HttpError): string => {
  switch (error.kind) {
    case 'http':
      return error.body ? `HTTP ${error.status} — ${error.body}` : `HTTP ${error.status}`
    case 'network':
      return error.message
    case 'decode':
      return `decode: ${error.message}`
  }
}

export { asyncFailure }
