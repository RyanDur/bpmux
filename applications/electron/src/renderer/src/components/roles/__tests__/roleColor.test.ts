import { describe, expect, test } from 'vitest'
import { roleColorFor } from '../roleColor'

describe('roleColorFor — dynamic role palette ', () => {
  test('returns saturated HSL for any role short-name including new ones (e.g. ops)', () => {
    const c = roleColorFor('ops')
    expect(c.color).toMatch(/^hsl\(\d+(?:\.\d+)? 70% 55%\)$/)
    expect(c.background).toMatch(/^hsla\(\d+(?:\.\d+)?, 70%, 55%, 0\.15\)$/)
  })

  test('same role short-name produces identical color across calls (deterministic hash)', () => {
    const a = roleColorFor('ops')
    const b = roleColorFor('ops')
    expect(a.color).toBe(b.color)
    expect(a.background).toBe(b.background)
  })

  test('different role short-names produce different hues for common bpmux team', () => {
    const colors = new Set([
      roleColorFor('owner').color,
      roleColorFor('dev').color,
      roleColorFor('pm').color,
      roleColorFor('designer').color,
      roleColorFor('ops').color
    ])
    expect(colors.size).toBe(5)
  })

  test('hue stays inside 0..359 for arbitrarily-shaped role names', () => {
    for (const role of ['', 'a', 'qa', 'observability', 'agent-abcd1234']) {
      const m = /^hsl\((\d+(?:\.\d+)?) 70% 55%\)$/.exec(roleColorFor(role).color)
      expect(m).not.toBeNull()
      const hue = Number(m![1])
      expect(hue).toBeGreaterThanOrEqual(0)
      expect(hue).toBeLessThan(360)
    }
  })
})
