import { type Result, asyncFailure, asyncResult, failure, success } from '@ryandur/sand'

export const toError = (e: unknown): Error => (e instanceof Error ? e : new Error(String(e)))

export const tryCatch = <S, F>(fn: () => S, onError: (error: unknown) => F): Result<S, F> => {
  try {
    return success(fn())
  } catch (e) {
    return failure(onError(e))
  }
}

export const asyncTryCatch = <S>(fn: () => Promise<S>): Result.Async<S, Error> =>
  tryCatch<Promise<S>, Error>(fn, toError).either<Result.Async<S, Error>, Result.Async<S, Error>>(
    (pending) => asyncResult<S, Error>(pending),
    asyncFailure
  )
