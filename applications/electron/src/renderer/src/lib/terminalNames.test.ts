import { describe, expect, test } from 'vitest'
import { resolveTerminalName, readTerminalNames, writeTerminalNames } from './terminalNames'

describe('#1609 terminal names — custom overrides the auto dir, clearing falls back', () => {
  test('a custom name overrides the auto dir', () => {
    expect(resolveTerminalName('build', 'bpmux')).toBe('build')
  })

  test('no custom name falls back to the auto dir', () => {
    expect(resolveTerminalName(undefined, 'bpmux')).toBe('bpmux')
  })

  test('an empty custom name falls back to the auto dir (cleared → auto)', () => {
    expect(resolveTerminalName('', 'bpmux')).toBe('bpmux')
  })

  test('names round-trip through storage keyed by terminal_id', () => {
    writeTerminalNames('ryan', { 'term-id-1': 'build', 'term-id-2': 'logs' })
    expect(readTerminalNames('ryan')).toEqual({ 'term-id-1': 'build', 'term-id-2': 'logs' })
  })

  test('reading a never-written identity yields an empty map', () => {
    expect(readTerminalNames('nobody-here')).toEqual({})
  })

  test('empty-string and non-string name entries are dropped on read', () => {
    window.localStorage.setItem(
      'bpmux-terminal-names:ryan',
      JSON.stringify({ keep: 'build', empty: '', bad: 7 })
    )
    expect(readTerminalNames('ryan')).toEqual({ keep: 'build' })
  })
})
