import { describe, expect, test } from 'vitest'
import { cruise } from 'dependency-cruiser'
import config from '../../../../.dependency-cruiser.cjs'

describe('dependency-cruiser substrate — the architecture rules genuinely fire', () => {
  test('no-circular fires on a two-module cycle', async () => {
    const outcome = await cruise(['src/renderer/src/test-support/fixtures/dependency-cycle'], {
      ruleSet: { forbidden: config.forbidden },
      validate: true
    })
    const output = outcome.output
    if (typeof output === 'string') throw new Error('expected structured cruise output')
    const fired = output.summary.violations.map((violation) => violation.rule.name)
    expect(fired).toContain('no-circular')
  })

  test('the live tree holds zero violations — the rules describe the app as it is', async () => {
    const outcome = await cruise(['src'], {
      ruleSet: { forbidden: config.forbidden },
      validate: true,
      doNotFollow: { path: 'node_modules' },
      tsConfig: { fileName: 'tsconfig.web.json' },
      exclude: { path: 'test-support/fixtures/dependency-cycle' }
    })
    const output = outcome.output
    if (typeof output === 'string') throw new Error('expected structured cruise output')
    expect(output.summary.violations).toEqual([])
  })
})
