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

  test('playwright-e2e runs on self-hosted macos arm64 + npx playwright test --shard (#1291)', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/runs-on:\s*\[\s*self-hosted\s*,\s*macos\s*,\s*arm64\s*\]/)
    expect(matrixSection).toMatch(/playwright test/)
    expect(matrixSection).toMatch(/--shard=/)
    expect(matrixSection).not.toMatch(/xvfb-run/)
  })

  test('playwright-e2e installs playwright browsers (no --with-deps on macOS; #1291)', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    expect(matrixSection).toMatch(/npx playwright install\b/)
    expect(matrixSection).not.toMatch(/--with-deps/)
  })

  test('playwright-e2e provisions bpmux binary via download-artifact OR local cargo build', () => {
    const yaml = ciYaml()
    const matrixSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    const buildsLocally = /cargo build/.test(matrixSection)
    const downloadsArtifact =
      /download-artifact/.test(matrixSection) && /name:\s*bpmux-binary/.test(matrixSection)
    expect(buildsLocally || downloadsArtifact).toBe(true)
  })

  test('a build-bpmux job exists OR playwright-e2e builds locally (one of two valid shapes per #1198)', () => {
    const yaml = ciYaml()
    const playwrightSection = yaml.match(/playwright-e2e:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
    const buildsLocally = /cargo build/.test(playwrightSection)
    const dedicatedBuildJob =
      /^\s*build-bpmux:\s*$/m.test(yaml) &&
      /actions\/upload-artifact[\s\S]*?name:\s*bpmux-binary/.test(yaml)
    expect(buildsLocally || dedicatedBuildJob).toBe(true)
  })

  test('playwright.config.ts does not testIgnore `playwright/**` (#1171 closes the coverage gap left by #1152 round 1)', () => {
    expect(existsSync(PLAYWRIGHT_CONFIG)).toBe(true)
    const config = playwrightConfig()
    expect(config).not.toMatch(/testIgnore:\s*['"]playwright\/\*\*['"]/)
  })
})
