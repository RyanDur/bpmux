import { test, expect } from '@playwright/test'
import { readFileSync, existsSync } from 'fs'
import { join } from 'path'

const CI_YAML = join(__dirname, '..', '..', '..', '.github', 'workflows', 'ci.yml')
const E2E_YAML = join(__dirname, '..', '..', '..', '.github', 'workflows', 'e2e.yml')
const PLAYWRIGHT_CONFIG = join(__dirname, '..', 'playwright.config.ts')

const ciYaml = (): string => readFileSync(CI_YAML, 'utf-8')
const e2eYaml = (): string => readFileSync(E2E_YAML, 'utf-8')
const playwrightConfig = (): string => readFileSync(PLAYWRIGHT_CONFIG, 'utf-8')

test.describe('Playwright runs in CI as a structural gate (#1152, post-#1463 split)', () => {
  test('e2e.yml exists at expected path', () => {
    expect(existsSync(E2E_YAML)).toBe(true)
  })

  test('e2e.yml declares a playwright-e2e job', () => {
    expect(e2eYaml()).toMatch(/^\s*playwright-e2e:\s*$/m)
  })

  test('playwright-e2e matrix splits into 6 shards on ubuntu-latest under xvfb', () => {
    const yaml = e2eYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*$/)?.[0] ?? ''
    expect(matrixSection).toMatch(/runs-on:\s*ubuntu-latest/)
    expect(matrixSection).toMatch(/shard:\s*\[\s*1\s*,\s*2\s*,\s*3\s*,\s*4\s*,\s*5\s*,\s*6\s*\]/)
    expect(matrixSection).toMatch(/xvfb-run/)
    expect(matrixSection).toMatch(/--shard=/)
  })

  test('playwright-e2e installs playwright browsers with --with-deps on Linux', () => {
    const yaml = e2eYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*$/)?.[0] ?? ''
    expect(matrixSection).toMatch(/playwright install.*--with-deps|--with-deps.*playwright install/)
  })

  test('e2e.yml on-block is workflow_dispatch + push:tags:v* only (#1463 substrate-fix; no schedule cron, no [e2e] commit-msg escape-hatch)', () => {
    const yaml = e2eYaml()
    const onBlock = yaml.match(/^on:[\s\S]*?(?=\njobs:)/m)?.[0] ?? ''
    expect(onBlock).toMatch(/workflow_dispatch:/)
    expect(onBlock).toMatch(/push:\s*[\s\S]*?tags:\s*[\s\S]*?["']v\*["']/)
    expect(onBlock).not.toMatch(/schedule:/)
  })

  test('e2e.yml does NOT carry a per-job if-gate (workflow-level trigger replaces it)', () => {
    const yaml = e2eYaml()
    const playwrightSection = yaml.match(/playwright-e2e:[\s\S]*$/)?.[0] ?? ''
    expect(playwrightSection).not.toMatch(/github\.event_name\s*==\s*'schedule'/)
    expect(playwrightSection).not.toMatch(
      /contains\(github\.event\.head_commit\.message,\s*'\[e2e\]'\)/
    )
  })

  test('playwright-e2e provisions bpmux binary via download-artifact', () => {
    const yaml = e2eYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*$/)?.[0] ?? ''
    expect(matrixSection).toMatch(/download-artifact/)
    expect(matrixSection).toMatch(/name:\s*bpmux-binary/)
  })

  test('a build-bpmux job exists in e2e.yml and uploads the bpmux-binary artifact', () => {
    const yaml = e2eYaml()
    expect(yaml).toMatch(/^\s*build-bpmux:\s*$/m)
    expect(yaml).toMatch(/actions\/upload-artifact[\s\S]*?name:\s*bpmux-binary/)
  })

  test('ci.yml no longer carries build-bpmux or playwright-e2e jobs (#1463 split)', () => {
    const yaml = ciYaml()
    expect(yaml).not.toMatch(/^\s*build-bpmux:\s*$/m)
    expect(yaml).not.toMatch(/^\s*playwright-e2e:\s*$/m)
  })

  test('ci.yml no longer carries a schedule trigger (#1463 retire daily cron)', () => {
    const yaml = ciYaml()
    const onBlock = yaml.match(/^on:[\s\S]*?(?=\njobs:)/m)?.[0] ?? ''
    expect(onBlock).not.toMatch(/schedule:/)
  })

  test('playwright.config.ts does not testIgnore `playwright/**` (#1171 closes the coverage gap left by #1152 round 1)', () => {
    expect(existsSync(PLAYWRIGHT_CONFIG)).toBe(true)
    const config = playwrightConfig()
    expect(config).not.toMatch(/testIgnore:\s*['"]playwright\/\*\*['"]/)
  })
})
