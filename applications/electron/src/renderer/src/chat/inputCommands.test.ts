import { describe, expect, test } from 'vitest'
import {
  lineEnd,
  lineStart,
  nextWordBoundary,
  parseAddressee,
  prevWordBoundary
} from './inputCommands'

describe('parseAddressee', () => {
  test('extracts a valid leading @role, case-insensitively', () => {
    expect(parseAddressee('@pm hello')).toBe('pm')
    expect(parseAddressee('@Dev hi')).toBe('dev')
  })

  test('returns null for an unknown role, a non-leading mention, or no mention', () => {
    expect(parseAddressee('@nobody hi')).toBeNull()
    expect(parseAddressee('hello @pm')).toBeNull()
    expect(parseAddressee('no mention here')).toBeNull()
  })
})

describe('word boundaries', () => {
  test('prevWordBoundary jumps to the start of the previous word', () => {
    expect(prevWordBoundary('hello world', 11)).toBe(6)
    expect(prevWordBoundary('hello world', 6)).toBe(0)
  })

  test('nextWordBoundary jumps to the end of the next word', () => {
    expect(nextWordBoundary('hello world', 0)).toBe(5)
    expect(nextWordBoundary('hello world', 5)).toBe(11)
  })
})

describe('line boundaries', () => {
  test('lineStart/lineEnd find the bounds of the current line', () => {
    const value = 'one\ntwo\nthree'
    expect(lineStart(value, 5)).toBe(4)
    expect(lineEnd(value, 5)).toBe(7)
    expect(lineStart(value, 0)).toBe(0)
    expect(lineEnd(value, 13)).toBe(13)
  })
})
