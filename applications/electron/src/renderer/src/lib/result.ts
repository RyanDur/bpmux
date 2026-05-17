/**
 * `Result<S, F>` — discriminated-union over success or failure with
 * monadic ops. Ported from the battleship repo's canonical Result
 * pattern (#1352 — Ryan-confirmed reference shape).
 *
 * `success` and `failure` are the only constructors; both return
 * frozen objects so misuse via mutation fails loudly.
 *
 * Use `andThen` to chain `Result`-returning steps without try-catch;
 * `or` to recover via a fallback `Result`; `either` / `mapEither` to
 * collapse the union into a single typed value at the boundary.
 */

export type Result<S, F> = Success<S, F> | Failure<S, F>

export type Success<S, F> = {
  readonly kind: 'success'
  readonly value: S
  map<T>(fn: (value: S) => T): Success<T, F>
  andThen<T, F2>(fn: (value: S) => Result<T, F2>): Result<T, F2>
  or<F2>(fn: (reason: F) => Result<S, F2>): Success<S, F2>
  onSuccess(fn: (value: S) => void): Success<S, F>
  onFailure(fn: (reason: F) => void): Success<S, F>
  either<S2, F2>(
    onSuccess: (value: S) => Result<S2, F2>,
    onFailure: (reason: F) => Result<S2, F2>
  ): Result<S2, F2>
  mapEither<T>(onSuccess: (value: S) => T, onFailure: (reason: F) => T): T
}

export type Failure<S, F> = {
  readonly kind: 'failure'
  readonly reason: F
  map<T>(fn: (value: S) => T): Failure<T, F>
  andThen<T, F2>(fn: (value: S) => Result<T, F2>): Failure<T, F>
  or<F2>(fn: (reason: F) => Result<S, F2>): Result<S, F2>
  onSuccess(fn: (value: S) => void): Failure<S, F>
  onFailure(fn: (reason: F) => void): Failure<S, F>
  either<S2, F2>(
    onSuccess: (value: S) => Result<S2, F2>,
    onFailure: (reason: F) => Result<S2, F2>
  ): Result<S2, F2>
  mapEither<T>(onSuccess: (value: S) => T, onFailure: (reason: F) => T): T
}

export const success = <S, F = never>(value: S): Success<S, F> =>
  Object.freeze({
    kind: 'success' as const,
    value,
    map: (fn) => success(fn(value)),
    andThen: (fn) => fn(value),
    or: () => success(value),
    onSuccess: (fn) => {
      fn(value)
      return success(value)
    },
    onFailure: () => success(value),
    either: (onSuccess) => onSuccess(value),
    mapEither: (onSuccess) => onSuccess(value)
  })

export const failure = <F, S = never>(reason: F): Failure<S, F> =>
  Object.freeze({
    kind: 'failure' as const,
    reason,
    map: () => failure(reason),
    andThen: () => failure(reason),
    or: (fn) => fn(reason),
    onSuccess: () => failure(reason),
    onFailure: (fn) => {
      fn(reason)
      return failure(reason)
    },
    either: (_onSuccess, onFailure) => onFailure(reason),
    mapEither: (_onSuccess, onFailure) => onFailure(reason)
  })

export const toError = (e: unknown): Error =>
  e instanceof Error ? e : new Error(String(e))

export const tryCatch = <S, F>(
  fn: () => S,
  onError: (error: unknown) => F
): Result<S, F> => {
  try {
    return success(fn())
  } catch (e) {
    return failure(onError(e))
  }
}
