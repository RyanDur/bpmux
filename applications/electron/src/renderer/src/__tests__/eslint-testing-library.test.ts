import { describe, test, expect } from 'vitest'
import { ESLint } from 'eslint'
import { resolve } from 'path'

const ELECTRON_APP_ROOT = resolve(__dirname, '../../../..')
const FIXTURE_PATH = resolve(__dirname, 'fixtures/dodds-anti-patterns.fixture.tsx')

const EXPECTED_RULE_IDS = [
  'testing-library/prefer-user-event',
  'testing-library/prefer-screen-queries',
  'testing-library/no-container',
  'testing-library/no-unnecessary-act',
  'testing-library/prefer-find-by',
  'testing-library/no-debugging-utils',
  'jest-dom/prefer-enabled-disabled'
] as const

const lintFixture = async (): Promise<ESLint.LintResult[]> => {
  const eslint = new ESLint({ cwd: ELECTRON_APP_ROOT, ignore: false })
  return eslint.lintFiles([FIXTURE_PATH])
}

describe('Dodds RTL anti-pattern eslint substrate (#1423)', () => {
  test.each(EXPECTED_RULE_IDS)('rule %s fires on its anti-pattern in fixture', async (ruleId) => {
    const [result] = await lintFixture()
    const firedRuleIds = result.messages.map((m) => m.ruleId)
    expect(firedRuleIds).toContain(ruleId)
  })

  test('grace-tier: rules emit warnings not errors', async () => {
    const [result] = await lintFixture()
    expect(result.errorCount).toBe(0)
    expect(result.warningCount).toBeGreaterThan(0)
  })
})
