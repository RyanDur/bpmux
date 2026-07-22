import { describe, test, expect } from 'vitest'
import stylelint from 'stylelint'
import { resolve } from 'path'

const ELECTRON_APP_ROOT = resolve(__dirname, '../../../..')
const CONFIG_FILE = resolve(ELECTRON_APP_ROOT, '.stylelintrc.json')
const APP_CSS_STUB = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/__tag-selector-stub__.css')
const BASE_LAYER_STUB = resolve(
  ELECTRON_APP_ROOT,
  'src/renderer/src/styles/__tag-selector-stub__.css'
)

const typeWarnings = async (code: string, codeFilename: string): Promise<stylelint.Warning[]> => {
  const { results } = await stylelint.lint({ code, codeFilename, configFile: CONFIG_FILE })
  return results[0].warnings.filter((w) => w.rule === 'selector-max-type')
}

describe('#1502 Stylelint tag-selector substrate — bare tag-selectors banned in app CSS', () => {
  test('app CSS: bare `details {}` fires selector-max-type (the #1516/#1517 disclosure case)', async () => {
    const warnings = await typeWarnings('details {\n  color: red;\n}\n', APP_CSS_STUB)
    expect(warnings.length).toBeGreaterThan(0)
    expect(warnings[0].severity).toBe('error')
  })

  test('app CSS: `details::details-content {}` fires (the owner 11:14 UTC anti-pattern)', async () => {
    const warnings = await typeWarnings(
      'details::details-content {\n  height: 0;\n}\n',
      APP_CSS_STUB
    )
    expect(warnings.length).toBeGreaterThan(0)
  })

  test('app CSS: intent-classname `.disclosure[open] {}` passes', async () => {
    const warnings = await typeWarnings('.disclosure[open] {\n  height: auto;\n}\n', APP_CSS_STUB)
    expect(warnings).toEqual([])
  })

  test('app CSS: class-anchored descendant tag (library-content scope) passes', async () => {
    const warnings = await typeWarnings(
      '.markdown-body h1 {\n  font-size: 2rem;\n}\n',
      APP_CSS_STUB
    )
    expect(warnings).toEqual([])
  })

  test('base-layer (styles/*.css): bare tag-selectors exempt (reset-first-in-specificity)', async () => {
    const warnings = await typeWarnings('button {\n  cursor: pointer;\n}\n', BASE_LAYER_STUB)
    expect(warnings).toEqual([])
  })
})
