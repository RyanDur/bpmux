import {
  Result,
  asyncFailure,
  asyncResult,
  asyncSuccess,
  asyncTryCatch,
  requesting
} from '@ryandur/sand'

export interface ApiContext {
  apiBase: string
  authToken: string | null | undefined
  ownerNonce?: string | null | undefined
}

export type HttpError =
  | { kind: 'http'; status: number; body: string }
  | { kind: 'network'; message: string }
  | { kind: 'decode'; message: string }

const STATUS_OK = [200]
const STATUS_OK_NO_CONTENT = [200, 201, 204]

const buildHeaders = (ctx: ApiContext): Record<string, string> => {
  const headers: Record<string, string> = {
    Accept: 'application/json',
    Authorization: `Bearer ${ctx.authToken ?? ''}`
  }
  if (ctx.ownerNonce) {
    headers['X-BPMUX-Owner-Nonce'] = ctx.ownerNonce
  }
  return headers
}

const networkError = (thrown: unknown): HttpError => ({
  kind: 'network',
  message: thrown instanceof Error ? thrown.message : String(thrown)
})

const settle = <T>(response: Response, successStatuses: number[]): Result.Async<T, HttpError> => {
  if (!successStatuses.includes(response.status)) {
    return asyncResult<string, HttpError>(response.text().catch(() => '')).mBind((body) =>
      asyncFailure<HttpError, T>({ kind: 'http', status: response.status, body })
    )
  }
  if (response.status === 204) {
    return asyncSuccess<T, HttpError>(undefined as unknown as T)
  }
  return asyncTryCatch(
    (): Promise<T> => response.json(),
    (thrown): HttpError => ({
      kind: 'decode',
      message: thrown instanceof Error ? thrown.message : String(thrown)
    })
  )
}

const request = <T>(
  ctx: ApiContext,
  endpoint: string,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
  body: unknown,
  successStatuses: number[],
  signal?: AbortSignal
): Result.Async<T, HttpError> =>
  requesting(
    `${ctx.apiBase}${endpoint}`,
    { method, headers: buildHeaders(ctx), body, ...(signal ? { signal } : {}) },
    networkError
  ).mBind((response) => settle<T>(response, successStatuses))

export const http = {
  get: <T>(ctx: ApiContext, endpoint: string, signal?: AbortSignal): Result.Async<T, HttpError> =>
    request<T>(ctx, endpoint, 'GET', undefined, STATUS_OK, signal),

  post: <T = void>(ctx: ApiContext, endpoint: string, body?: unknown): Result.Async<T, HttpError> =>
    request<T>(ctx, endpoint, 'POST', body, STATUS_OK_NO_CONTENT),

  put: <T = void>(ctx: ApiContext, endpoint: string, body?: unknown): Result.Async<T, HttpError> =>
    request<T>(ctx, endpoint, 'PUT', body, STATUS_OK_NO_CONTENT),

  delete: <T = void>(
    ctx: ApiContext,
    endpoint: string,
    body?: unknown
  ): Result.Async<T, HttpError> => request<T>(ctx, endpoint, 'DELETE', body, STATUS_OK_NO_CONTENT)
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
