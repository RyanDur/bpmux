import { describe, test, expect } from 'vitest'
import { ESLint } from 'eslint'
import { resolve } from 'path'

const ELECTRON_APP_ROOT = resolve(__dirname, '../../../../..')
const STUB_PRODUCTION_TSX = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/__html5-stub__.tsx')
const STUB_TEST_TSX = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/__html5-stub__.test.tsx')

const lintAs = async (code: string, filePath: string): Promise<ESLint.LintResult[]> => {
  const eslint = new ESLint({
    cwd: ELECTRON_APP_ROOT,
    overrideConfig: [
      {
        files: ['**/*.{ts,tsx}'],
        languageOptions: { parserOptions: { projectService: false } },
        rules: { '@typescript-eslint/switch-exhaustiveness-check': 'off' }
      }
    ]
  })
  return eslint.lintText(code, { filePath })
}

const ruleFor = (
  msgs: ESLint.LintResult['messages'],
  ruleId: string
): ESLint.LintResult['messages'] => msgs.filter((m) => m.ruleId === ruleId)

describe('ESLint html5-semantic substrate — JSX div/span banned in renderer production', () => {
  test('production .tsx: <div> fires no-restricted-syntax error', async () => {
    const code = `export const Card = () => <div className="card">hi</div>\n`
    const [result] = await lintAs(code, STUB_PRODUCTION_TSX)
    const violations = ruleFor(result.messages, 'no-restricted-syntax')
    expect(violations.length).toBeGreaterThan(0)
    expect(violations[0].severity).toBe(2)
    expect(violations[0].message).toMatch(/<div>/)
    expect(violations[0].message).toMatch(/html5-semantic/)
  })

  test('production .tsx: <span> fires no-restricted-syntax error', async () => {
    const code = `export const Badge = () => <span className="badge">x</span>\n`
    const [result] = await lintAs(code, STUB_PRODUCTION_TSX)
    const violations = ruleFor(result.messages, 'no-restricted-syntax')
    expect(violations.length).toBeGreaterThan(0)
    expect(violations[0].severity).toBe(2)
    expect(violations[0].message).toMatch(/<span>/)
    expect(violations[0].message).toMatch(/feedback_first_rule_of_aria/)
  })

  test('production .tsx: html5 semantic elements pass the rule', async () => {
    const code = `export const Card = () => (
  <article>
    <header><h2>title</h2></header>
    <section>body</section>
  </article>
)
`
    const [result] = await lintAs(code, STUB_PRODUCTION_TSX)
    const violations = ruleFor(result.messages, 'no-restricted-syntax')
    expect(violations).toEqual([])
  })

  test('test .tsx: <div>/<span> NOT banned (test files freer for shape-assertion)', async () => {
    const code = `export const Fixture = () => <div><span>x</span></div>\n`
    const [result] = await lintAs(code, STUB_TEST_TSX)
    const violations = ruleFor(result.messages, 'no-restricted-syntax')
    expect(violations).toEqual([])
  })
})
