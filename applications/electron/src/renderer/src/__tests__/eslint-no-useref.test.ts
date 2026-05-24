import { describe, test, expect } from 'vitest'
import { ESLint } from 'eslint'
import { resolve } from 'path'

const ELECTRON_APP_ROOT = resolve(__dirname, '../../../..')
const STUB_PRODUCTION_TSX = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/__useref_stub__.tsx')
const STUB_TEST_TSX = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/__useref_stub__.test.tsx')
const WAIVED_FILE = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/board/BoardResizeHandle.tsx')
const WAIVED_TSX = resolve(ELECTRON_APP_ROOT, 'src/renderer/src/board/Comments.tsx')
const UNWAIVED_1537_FILES = [
  'src/renderer/src/transport/useWebSocket.ts',
  'src/renderer/src/board/useRefetchOnReconnect.ts',
  'src/renderer/src/hooks/useNotifications.ts'
].map((f) => resolve(ELECTRON_APP_ROOT, f))

const lintAs = async (code: string, filePath: string): Promise<ESLint.LintResult[]> => {
  const eslint = new ESLint({ cwd: ELECTRON_APP_ROOT })
  return eslint.lintText(code, { filePath })
}

const ruleFor = (
  msgs: ESLint.LintResult['messages'],
  ruleId: string
): ESLint.LintResult['messages'] => msgs.filter((m) => m.ruleId === ruleId)

const refViolations = (msgs: ESLint.LintResult['messages']): ESLint.LintResult['messages'] =>
  ruleFor(msgs, 'no-restricted-syntax').filter((m) => /useRef/.test(m.message))

describe('#1499 ESLint no-useRef substrate — Ryan-canon "no refs unless no other way"', () => {
  test('production file: useRef fires no-restricted-syntax error', async () => {
    const code = `import { useRef } from 'react'\nexport const C = () => { const r = useRef(0); return r.current }\n`
    const [result] = await lintAs(code, STUB_PRODUCTION_TSX)
    const violations = refViolations(result.messages)
    expect(violations.length).toBeGreaterThan(0)
    expect(violations[0].severity).toBe(2)
    expect(violations[0].message).toMatch(/no other way/)
  })

  test('production file: useState (no ref) passes the rule', async () => {
    const code = `import { useState } from 'react'\nexport const C = () => { const [n] = useState(0); return n }\n`
    const [result] = await lintAs(code, STUB_PRODUCTION_TSX)
    expect(refViolations(result.messages)).toEqual([])
  })

  test('waived file (BoardResizeHandle.tsx, until:#1538): useRef is allowed (conditional keep)', async () => {
    const code = `import { useRef } from 'react'\nexport const C = () => { const r = useRef(0); return r.current }\n`
    const [result] = await lintAs(code, WAIVED_FILE)
    expect(refViolations(result.messages)).toEqual([])
  })

  test('#1537: the formerly-waived WS files now ENFORCE no-useRef (waiver discharged)', async () => {
    const code = `import { useRef } from 'react'\nexport const C = () => { const r = useRef(0); return r.current }\n`
    for (const file of UNWAIVED_1537_FILES) {
      const [result] = await lintAs(code, file)
      expect(refViolations(result.messages).length).toBeGreaterThan(0)
    }
  })

  test('waived .tsx still enforces div/span (waiver drops only useRef, not html5-semantic)', async () => {
    const code = `export const C = () => <div>x</div>\n`
    const [result] = await lintAs(code, WAIVED_TSX)
    const divViolations = ruleFor(result.messages, 'no-restricted-syntax').filter((m) =>
      /<div>/.test(m.message)
    )
    expect(divViolations.length).toBeGreaterThan(0)
  })

  test('test file: useRef NOT banned (tests are exempt)', async () => {
    const code = `import { useRef } from 'react'\nexport const C = () => { const r = useRef(0); return r.current }\n`
    const [result] = await lintAs(code, STUB_TEST_TSX)
    expect(refViolations(result.messages)).toEqual([])
  })
})
