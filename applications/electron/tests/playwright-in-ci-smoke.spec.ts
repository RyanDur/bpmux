import { test, expect } from '@playwright/test'
import { readFileSync, existsSync } from 'fs'
import { join } from 'path'

const CI_YAML = join(__dirname, '..', '..', '..', '.github', 'workflows', 'ci.yml')
const PLAYWRIGHT_CONFIG = join(__dirname, '..', 'playwright.config.ts')

const ciYaml = (): string => readFileSync(CI_YAML, 'utf-8')
const playwrightConfig = (): string => readFileSync(PLAYWRIGHT_CONFIG, 'utf-8')

test.describe('Playwright runs in CI as a structural gate (#1152)', () => {
  test('ci.yml exists at expected path', () => {
    expect(existsSync(CI_YAML)).toBe(true)
  })

  test('ci.yml declares a playwright-e2e job', () => {
    expect(ciYaml()).toMatch(/^\s*playwright-e2e:\s*$/m)
  })

  test('playwright-e2e matrix splits into 6 shards', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/shard:\s*\[\s*1\s*,\s*2\s*,\s*3\s*,\s*4\s*,\s*5\s*,\s*6\s*\]/)
  })

  test('playwright-e2e job uses xvfb-run + npx playwright test --shard', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/xvfb-run/)
    expect(matrixSection).toMatch(/playwright test/)
    expect(matrixSection).toMatch(/--shard=/)
  })

  test('playwright-e2e installs playwright browsers with --with-deps', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/playwright install.*--with-deps|--with-deps.*playwright install/)
  })

  test('playwright-e2e builds bpmux Rust binary (Electron spawns target/{debug,release}/bpmux at runtime)', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/cargo build/)
  })

  test('playwright.config.ts does not testIgnore `playwright/**` (#1171 closes the coverage gap left by #1152 round 1)', () => {
    expect(existsSync(PLAYWRIGHT_CONFIG)).toBe(true)
    const config = playwrightConfig()
    expect(config).not.toMatch(/testIgnore:\s*['"]playwright\/\*\*['"]/)
  })
})
