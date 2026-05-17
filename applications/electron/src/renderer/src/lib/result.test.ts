import { describe, expect, it, vi } from 'vitest'

import { success, failure, tryCatch, toError, type Result } from './result'

describe('Result', () => {
  describe('success', () => {
    it('maps over the value', () => {
      const result = success(2).map((n) => n * 3)

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(6)
    })

    it('andThen chains into another Result on success', () => {
      const safeDivide = (n: number): Result<number, string> =>
        n === 0 ? failure('division by zero') : success(10 / n)

      const result = success(2).andThen(safeDivide)

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(5)
    })

    it('passes through or, ignoring the recovery function', () => {
      const result = success<number, string>(42).or(() => failure('recovered'))

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(42)
    })

    it('onSuccess runs a side effect and returns self', () => {
      const spy = vi.fn()
      const result = success(42).onSuccess(spy)

      expect(spy).toHaveBeenCalledWith(42)
      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(42)
    })

    it('onFailure skips side effect on success and returns self', () => {
      const spy = vi.fn()
      const result = success(42).onFailure(spy)

      expect(spy).not.toHaveBeenCalled()
      expect(result.kind).toBe('success')
    })

    it('either applies onSuccess and returns a Result', () => {
      const result = success<number, string>(42).either(
        (n) => success(`got ${n}`),
        () => failure('failed')
      )

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => ''
        )
      ).toBe('got 42')
    })

    it('folds with mapEither, calling onSuccess', () => {
      const value = success(42).mapEither(
        (n) => `got ${n}`,
        () => 'failed'
      )

      expect(value).toBe('got 42')
    })
  })

  describe('failure', () => {
    it('passes through map unchanged', () => {
      const result = failure<string>('oops').map((n: number) => n * 3)

      expect(result.kind).toBe('failure')
      expect(
        result.mapEither(
          () => '',
          (reason) => reason
        )
      ).toBe('oops')
    })

    it('andThen passes through unchanged on failure', () => {
      const result = failure<string>('oops').andThen(() => success(42))

      expect(result.kind).toBe('failure')
    })

    it('recovers with or, applying the recovery function', () => {
      const result = failure<string, number>('oops').or((reason) => success(reason.length))

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(4)
    })

    it('can recover to another failure with or', () => {
      const result = failure<string, number>('oops').or(() => failure(42))

      expect(result.kind).toBe('failure')
      expect(
        result.mapEither(
          () => 0,
          (v) => v
        )
      ).toBe(42)
    })

    it('onSuccess skips side effect on failure and passes through', () => {
      const spy = vi.fn()
      const result = failure<string>('oops').onSuccess(spy)

      expect(spy).not.toHaveBeenCalled()
      expect(result.kind).toBe('failure')
    })

    it('onFailure runs a side effect and returns self', () => {
      const spy = vi.fn()
      const result = failure<string, number>('oops').onFailure(spy)

      expect(spy).toHaveBeenCalledWith('oops')
      expect(result.kind).toBe('failure')
    })

    it('either applies onFailure and returns a Result', () => {
      const result = failure<string, number>('oops').either(
        () => success(0),
        (reason) => success(reason.length)
      )

      expect(result.kind).toBe('success')
      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(4)
    })

    it('folds with mapEither, calling onFailure', () => {
      const value = failure('oops').mapEither(
        () => 'succeeded',
        (reason) => `failed: ${reason}`
      )

      expect(value).toBe('failed: oops')
    })
  })

  describe('chaining', () => {
    it('short-circuits on first failure in an andThen chain', () => {
      const parse = (s: string): Result<number, string> => {
        const n = Number(s)
        return isNaN(n) ? failure('not a number') : success(n)
      }

      const result = success('abc')
        .andThen(parse)
        .map((n) => n * 2)

      expect(result.kind).toBe('failure')
      expect(
        result.mapEither(
          () => '',
          (reason) => reason
        )
      ).toBe('not a number')
    })

    it('composes map and andThen through a pipeline', () => {
      const parse = (s: string): Result<number, string> => {
        const n = Number(s)
        return isNaN(n) ? failure('not a number') : success(n)
      }

      const result = success('21')
        .andThen(parse)
        .map((n) => n * 2)

      expect(
        result.mapEither(
          (v) => v,
          () => 0
        )
      ).toBe(42)
    })
  })

  describe('toError', () => {
    it('returns an Error unchanged when given an Error', () => {
      const error = new Error('original')
      expect(toError(error)).toBe(error)
    })

    it('wraps a non-Error value in a new Error', () => {
      expect(toError('something went wrong').message).toBe('something went wrong')
    })
  })

  describe('tryCatch', () => {
    it('wraps a successful computation in success', () => {
      const result = tryCatch(
        () => JSON.parse('{"x":1}'),
        () => 'parse error'
      )

      expect(result.kind).toBe('success')
    })

    it('wraps a thrown exception in failure', () => {
      const result = tryCatch(
        () => JSON.parse('bad json'),
        (e) => `error: ${e}`
      )

      expect(result.kind).toBe('failure')
      expect(
        result.mapEither(
          () => '',
          (r) => r
        )
      ).toMatch(/error:/)
    })
  })
})
