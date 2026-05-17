import { describe, expect, it } from 'vitest'

import { maybe, nothing, some } from './maybe'

describe('Maybe', () => {
  it('some maps over the value', () => {
    const m = some(2).map((n) => n * 3)
    expect(m.isNothing).toBe(false)
    expect(m.orElse(0)).toBe(6)
  })

  it('some.mBind chains into another Maybe', () => {
    const m = some(2).mBind((n) => some(n + 1))
    expect(m.isNothing).toBe(false)
    expect(m.orElse(0)).toBe(3)
  })

  it('some.mBind can return nothing', () => {
    const m = some(2).mBind(() => nothing<number>())
    expect(m.isNothing).toBe(true)
  })

  it('some.or passes through', () => {
    const m = some(42).or(() => some(99))
    expect(m.orElse(0)).toBe(42)
  })

  it('some.orElse returns the value', () => {
    expect(some(42).orElse(0)).toBe(42)
  })

  it('some.orNull returns the value', () => {
    expect(some(42).orNull()).toBe(42)
  })

  it('nothing.map returns nothing', () => {
    const m = nothing<number>().map((n) => n * 3)
    expect(m.isNothing).toBe(true)
  })

  it('nothing.mBind returns nothing', () => {
    const m = nothing<number>().mBind(() => some(42))
    expect(m.isNothing).toBe(true)
  })

  it('nothing.or applies the fallback fn', () => {
    const m = nothing<number>().or(() => some(99))
    expect(m.orElse(0)).toBe(99)
  })

  it('nothing.orElse returns the fallback', () => {
    expect(nothing<number>().orElse(42)).toBe(42)
  })

  it('nothing.orNull returns null', () => {
    expect(nothing().orNull()).toBe(null)
  })

  it('maybe(null) returns nothing', () => {
    expect(maybe(null).isNothing).toBe(true)
  })

  it('maybe(undefined) returns nothing', () => {
    expect(maybe(undefined).isNothing).toBe(true)
  })

  it('maybe(value) returns some(value)', () => {
    const m = maybe(42)
    expect(m.isNothing).toBe(false)
    expect(m.orElse(0)).toBe(42)
  })
})
