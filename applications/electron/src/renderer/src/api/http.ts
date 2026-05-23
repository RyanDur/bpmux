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

const buildHeaders = (ctx: ApiContext, includeContentType: boolean): Record<string, string> => {
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
  get: <T>(ctx: ApiContext, endpoint: string, signal?: AbortSignal): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'GET', undefined, STATUS_OK, signal)),

  post: <T = void>(ctx: ApiContext, endpoint: string, body?: unknown): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'POST', body, STATUS_OK_NO_CONTENT)),

  put: <T = void>(ctx: ApiContext, endpoint: string, body?: unknown): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'PUT', body, STATUS_OK_NO_CONTENT)),

  delete: <T = void>(
    ctx: ApiContext,
    endpoint: string,
    body?: unknown
  ): AsyncResult<T, HttpError> =>
    ofPromise(request<T>(ctx, endpoint, 'DELETE', body, STATUS_OK_NO_CONTENT))
}

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
