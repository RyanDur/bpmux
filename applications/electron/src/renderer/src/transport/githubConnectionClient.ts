import * as D from 'schemawax'
import { type Result } from '@ryandur/sand'
import { type ApiContext, type HttpError, http } from './http'
import { validate } from './validate'

export type GithubConnectionStatus =
  | { state: 'disconnected' }
  | { state: 'connected'; owner: string; repo: string; baseUrl?: string }

export type ConnectPayload = {
  owner: string
  repo: string
  token: string
}

const wire = D.object({
  required: { state: D.literalUnion('connected', 'disconnected') },
  optional: {
    owner: D.string,
    repo: D.string,
    base_url: D.string
  }
})

const statusDecoder: D.Decoder<GithubConnectionStatus> = wire.andThen((decoded) =>
  decoded.state === 'connected' && decoded.owner !== undefined && decoded.repo !== undefined
    ? {
        state: 'connected' as const,
        owner: decoded.owner,
        repo: decoded.repo,
        ...(decoded.base_url !== undefined ? { baseUrl: decoded.base_url } : {})
      }
    : { state: 'disconnected' as const }
)

export const getGithubConnection = (
  ctx: ApiContext
): Result.Async<GithubConnectionStatus, HttpError> =>
  http.get<unknown>(ctx, '/api/github-connection').mBind(validate(statusDecoder))

export const putGithubConnection = (
  ctx: ApiContext,
  payload: ConnectPayload
): Result.Async<GithubConnectionStatus, HttpError> =>
  http.put<unknown>(ctx, '/api/github-connection', payload).mBind(validate(statusDecoder))

export const deleteGithubConnection = (
  ctx: ApiContext
): Result.Async<GithubConnectionStatus, HttpError> =>
  http.delete<unknown>(ctx, '/api/github-connection').mBind(validate(statusDecoder))
