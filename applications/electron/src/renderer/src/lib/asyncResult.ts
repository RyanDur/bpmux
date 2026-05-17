/**
 * `AsyncResult<S, F>` — `Promise<Result<S, F>>` wrapped in a monadic
 * interface so callers can chain `.map` / `.andThen` / `.or` across
 * async boundaries without unwrapping into try-catch. Ported from
 * the battleship repo's canonical pattern (#1352).
 *
 * Use `asyncResult(promise)` to lift a regular `Promise<S>` into an
 * `AsyncResult<S, F>` (catches into `failure`); `asyncTryCatch` for
 * the same with `Error` as the failure type; `asyncSuccess` /
 * `asyncFailure` to construct directly; `ofPromise` to wrap a
 * `Promise<Result<S, F>>` someone else built.
 */

import { success, failure, tryCatch, toError, type Result } from './result'

export type AsyncResult<S, F> = {
  readonly value: Promise<Result<S, F>>
  map<T>(fn: (value: S) => T): AsyncResult<T, F>
  andThen<T>(fn: (value: S) => AsyncResult<T, F>): AsyncResult<T, F>
  or<F2>(fn: (reason: F) => AsyncResult<S, F2>): AsyncResult<S, F2>
  either<S2, F2>(
    onSuccess: (value: S) => AsyncResult<S2, F2>,
    onFailure: (reason: F) => AsyncResult<S2, F2>
  ): AsyncResult<S2, F2>
  onSuccess(fn: (value: S) => void): AsyncResult<S, F>
  onFailure(fn: (reason: F) => void): AsyncResult<S, F>
  onComplete(fn: (result: Result<S, F>) => void): AsyncResult<S, F>
  onPending(fn: (pending: boolean) => void): AsyncResult<S, F>
  mapEither<T>(onSuccess: (value: S) => T, onFailure: (reason: F) => T): Promise<T>
}

export const ofPromise = <S, F>(promise: Promise<Result<S, F>>): AsyncResult<S, F> => ({
  value: promise,
  map: <T>(fn: (value: S) => T): AsyncResult<T, F> =>
    ofPromise(promise.then((result) => result.map(fn))),
  andThen: <T>(fn: (value: S) => AsyncResult<T, F>): AsyncResult<T, F> =>
    ofPromise<T, F>(
      promise.then((result): Promise<Result<T, F>> => {
        if (result.kind === 'success') return fn(result.value).value
        return Promise.resolve(failure<F, T>(result.reason))
      })
    ),
  or: <F2>(fn: (reason: F) => AsyncResult<S, F2>): AsyncResult<S, F2> =>
    ofPromise<S, F2>(
      promise.then((result): Promise<Result<S, F2>> => {
        if (result.kind === 'failure') return fn(result.reason).value
        return Promise.resolve(success<S, F2>(result.value))
      })
    ),
  either: <S2, F2>(
    onSuccess: (value: S) => AsyncResult<S2, F2>,
    onFailure: (reason: F) => AsyncResult<S2, F2>
  ): AsyncResult<S2, F2> =>
    ofPromise<S2, F2>(
      promise.then(
        (result): Promise<Result<S2, F2>> =>
          result.mapEither(
            (value) => onSuccess(value).value,
            (reason) => onFailure(reason).value
          )
      )
    ),
  onSuccess: (fn) => ofPromise(promise.then((result) => result.onSuccess(fn))),
  onFailure: (fn) => ofPromise(promise.then((result) => result.onFailure(fn))),
  onComplete: (fn) =>
    ofPromise(
      promise.then((result) => {
        fn(result)
        return result
      })
    ),
  onPending: (fn) => {
    fn(true)
    return ofPromise(
      promise.then((result) => {
        fn(false)
        return result
      })
    )
  },
  mapEither: (onSuccess, onFailure) =>
    promise.then((result) => result.mapEither(onSuccess, onFailure))
})

export const asyncSuccess = <S, F = never>(value: S): AsyncResult<S, F> =>
  ofPromise(Promise.resolve(success(value)))

export const asyncFailure = <F, S = never>(reason: F): AsyncResult<S, F> =>
  ofPromise(Promise.resolve(failure(reason)))

export const asyncResult = <S, F>(promise: Promise<S>): AsyncResult<S, F> =>
  ofPromise(
    promise.then((value) => success<S, F>(value)).catch((error) => failure<F, S>(error as F))
  )

export const asyncTryCatch = <S>(fn: () => Promise<S>): AsyncResult<S, Error> =>
  tryCatch<Promise<S>, Error>(fn, toError).mapEither<AsyncResult<S, Error>>(
    asyncResult,
    asyncFailure
  )
