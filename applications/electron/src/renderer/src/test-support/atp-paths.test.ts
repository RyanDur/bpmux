import { describe, expect, test } from 'vitest'
import { readFileSync, readdirSync, existsSync } from 'fs'
import { join, resolve } from 'path'

const TESTS_DIR = resolve(__dirname, '../../../../tests')
const REPO_ROOT = resolve(__dirname, '../../../../../..')

describe('acceptance-test paths never lie — the rename hazard that bit three times stays dead', () => {
  test('no spec hardcodes its own path as a string literal — atpPath owns derivation', () => {
    const offenders: string[] = []
    for (const file of readdirSync(TESTS_DIR).filter((f) => f.endsWith('.spec.ts'))) {
      const src = readFileSync(join(TESTS_DIR, file), 'utf-8')
      if (/'applications\/electron\/tests\/[^']+\.spec\.ts'/.test(src)) {
        offenders.push(file)
      }
    }
    expect(offenders, 'derive with atpPath(__filename) instead').toEqual([])
  })

  test('every acceptance_test_path a spec can seed points at a file that exists', () => {
    const missing: string[] = []
    for (const file of readdirSync(TESTS_DIR).filter((f) => f.endsWith('.spec.ts'))) {
      const src = readFileSync(join(TESTS_DIR, file), 'utf-8')
      for (const match of src.matchAll(/'(applications\/electron\/tests\/[^']+)'/g)) {
        if (!existsSync(join(REPO_ROOT, match[1]))) {
          missing.push(`${file} → ${match[1]}`)
        }
      }
      if (src.includes('acceptance_test_path') && !src.includes('atpPath(__filename)')) {
        missing.push(`${file} seeds a story without deriving its ATP from atpPath`)
      }
    }
    expect(missing).toEqual([])
  })
})
