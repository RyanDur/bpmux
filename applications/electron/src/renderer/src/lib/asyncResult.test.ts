import { describe, expect, it } from 'vitest'

import {
  asyncFailure,
  asyncResult,
  asyncSuccess,
  asyncTryCatch,
  ofPromise,
  type AsyncResult
} from './asyncResult'
import { failure, success, type Result } from './result'

describe('AsyncResult', () => {
  it('asyncSuccess resolves with the success value', async () => {
    const result = await asyncSuccess(42).mapEither(
      (value) => value,
      () => 0
    )
    expect(result).toBe(42)
  })

  it('asyncFailure resolves with the failure reason', async () => {
    const result = await asyncFailure<string, number>('oops').mapEither(
      () => 0,
      (reason) => reason.length
    )
    expect(result).toBe(4)
  })

  it('asyncResult wraps a resolved promise as success', async () => {
    const result = await asyncResult<number, string>(Promise.resolve(42)).mapEither(
      (value) => value,
      () => 0
    )
    expect(result).toBe(42)
  })

  it('asyncResult wraps a rejected promise as failure', async () => {
    const result = await asyncResult<number, string>(Promise.reject('oops')).mapEither(
      () => 0,
      (reason) => reason.length
    )
    expect(result).toBe(4)
  })

  it('map transforms the success value', async () => {
    const result = await asyncSuccess(2)
      .map((value) => value * 3)
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(result).toBe(6)
  })

  it('andThen chains into another AsyncResult on success', async () => {
    const safeDivide = (divisor: number): AsyncResult<number, string> =>
      divisor === 0 ? asyncFailure('division by zero') : asyncSuccess(10 / divisor)

    const result = await asyncSuccess<number, string>(2)
      .andThen(safeDivide)
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(result).toBe(5)
  })

  it('andThen short-circuits on failure', async () => {
    const result = await asyncFailure<string, number>('oops')
      .andThen(() => asyncSuccess(42))
      .mapEither(
        () => '',
        (reason) => reason
      )
    expect(result).toBe('oops')
  })

  it('or recovers from failure', async () => {
    const result = await asyncFailure<string, number>('oops')
      .or(() => asyncSuccess(99))
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(result).toBe(99)
  })

  it('either applies onSuccess branch', async () => {
    const result = await asyncSuccess<number, string>(42)
      .either(
        (value) => asyncSuccess(`got ${value}`),
        () => asyncSuccess('failed')
      )
      .mapEither(
        (value) => value,
        () => ''
      )
    expect(result).toBe('got 42')
  })

  it('onSuccess runs a side effect on success and returns self', async () => {
    const log: number[] = []
    const result = await asyncSuccess(42)
      .onSuccess((value) => log.push(value))
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(log).toEqual([42])
    expect(result).toBe(42)
  })

  it('onFailure runs a side effect on failure and returns self', async () => {
    const log: string[] = []
    const result = await asyncFailure<string, number>('oops')
      .onFailure((reason) => log.push(reason))
      .mapEither(
        () => 0,
        (reason) => reason.length
      )
    expect(log).toEqual(['oops'])
    expect(result).toBe(4)
  })

  it('onComplete runs a side effect with the full Result', async () => {
    const log: Result<number, string>[] = []
    await asyncSuccess<number, string>(42)
      .onComplete((result) => log.push(result))
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(log).toHaveLength(1)
    expect(log[0].kind).toBe('success')
  })

  it('onPending calls callback with true immediately then false when resolved', async () => {
    const log: boolean[] = []
    const pending = asyncSuccess(42).onPending((isPending) => log.push(isPending))
    expect(log).toEqual([true])
    await pending.mapEither(
      (value) => value,
      () => 0
    )
    expect(log).toEqual([true, false])
  })

  it('ofPromise wraps a resolved success Result as success', async () => {
    const result = await ofPromise(Promise.resolve(success(42))).mapEither(
      (v) => v,
      () => 0
    )
    expect(result).toBe(42)
  })

  it('ofPromise wraps a resolved failure Result as failure', async () => {
    const result = await ofPromise(Promise.resolve(failure<string, number>('oops'))).mapEither(
      () => 0,
      (r) => r.length
    )
    expect(result).toBe(4)
  })

  it('asyncTryCatch wraps a resolved promise as success', async () => {
    const result = await asyncTryCatch(() => Promise.resolve(42)).mapEither(
      (value) => value,
      () => 0
    )
    expect(result).toBe(42)
  })

  it('asyncTryCatch wraps a rejected promise as failure', async () => {
    const result = await asyncTryCatch(() => Promise.reject(new Error('boom'))).mapEither(
      () => '',
      (e) => e.message
    )
    expect(result).toBe('boom')
  })

  it('asyncTryCatch catches a synchronous throw as failure', async () => {
    const result = await asyncTryCatch(() => {
      throw new Error('sync')
    }).mapEither(
      () => '',
      (e) => e.message
    )
    expect(result).toBe('sync')
  })

  it('composes map and andThen through a pipeline', async () => {
    const parse = (input: string): AsyncResult<number, string> => {
      const num = Number(input)
      return isNaN(num) ? asyncFailure('not a number') : asyncSuccess(num)
    }

    const result = await asyncSuccess<string, string>('21')
      .andThen(parse)
      .map((value) => value * 2)
      .mapEither(
        (value) => value,
        () => 0
      )
    expect(result).toBe(42)
  })
})
