import { describe, expect, test } from 'vitest'
import { composeTerminalFontFamily } from './terminalFont'

describe('#1610 composeTerminalFontFamily — name a Nerd Font in the chain (leverage system, no bundle)', () => {
  test('with no user --font-mono, MesloLGS NF is the floor before system mono', () => {
    expect(composeTerminalFontFamily('')).toBe(
      '"MesloLGS NF", ui-monospace, SFMono-Regular, Menlo, monospace'
    )
  })

  test('a user --font-mono is preferred for ASCII; MesloLGS NF is always appended for glyph coverage', () => {
    const chain = composeTerminalFontFamily('Fira Code')
    expect(chain.startsWith('Fira Code, ')).toBe(true)
    expect(chain).toContain('"MesloLGS NF"')
    expect(chain.indexOf('Fira Code')).toBeLessThan(chain.indexOf('"MesloLGS NF"'))
  })

  test('the Nerd Font is named by name — never bundled: no path, no @font-face url in the chain', () => {
    const chain = composeTerminalFontFamily('')
    expect(chain).not.toContain('url(')
    expect(chain).not.toContain('.ttf')
    expect(chain).not.toContain('/')
  })
})
