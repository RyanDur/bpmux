/**
 * `Maybe<T>` — `Some<T> | Nothing<T>` discriminated-union with
 * monadic ops. Ported from the battleship repo's canonical pattern
 * (#1352).
 *
 * Use `maybe(value)` to lift a nullable value into a `Maybe`; `some`
 * / `nothing` to construct directly. Chain via `.map` / `.mBind` /
 * `.or` without explicit null-checks; collapse via `.orElse(default)`
 * / `.orNull()` at the boundary.
 */

type Some<T> = {
  isNothing: false
  map<U>(fn: (value: T) => U): Some<U>
  mBind<U>(fn: (value: T) => Maybe<U>): Maybe<U>
  or(_fn: () => Maybe<T>): Some<T>
  orElse(_fallback: T): T
  orNull(): T
}

type Nothing<T> = {
  isNothing: true
  map<U>(_fn: (value: T) => U): Nothing<U>
  mBind<U>(_fn: (value: T) => Maybe<U>): Nothing<U>
  or(fn: () => Maybe<T>): Maybe<T>
  orElse(fallback: T): T
  orNull(): null
}

export type Maybe<T> = Some<T> | Nothing<T>

export const some = <T>(value: T): Some<T> =>
  Object.freeze({
    isNothing: false as const,
    map: (fn) => some(fn(value)),
    mBind: (fn) => fn(value),
    or: () => some(value),
    orElse: () => value,
    orNull: () => value
  })

export const nothing = <T = never>(): Nothing<T> =>
  Object.freeze({
    isNothing: true as const,
    map: () => nothing(),
    mBind: () => nothing(),
    or: (fn) => fn(),
    orElse: (fallback) => fallback,
    orNull: () => null
  })

export const maybe = <T>(value: T | null | undefined): Maybe<T> =>
  value !== undefined && value !== null ? some(value) : nothing()
