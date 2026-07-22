import { describe, expect, test } from 'vitest'
import * as D from 'schemawax'
import { validate } from './validate'

const decoder = D.object({ required: { n: D.number } })

describe('validate — decoder → AsyncResult boundary bridge', () => {
  test('valid input decodes through to a success', async () => {
    const result = await validate(decoder)({ n: 7 }).value
    expect(result.isSuccess).toBe(true)
    if (result.isSuccess) expect(result.value.n).toBe(7)
  })

  test('invalid input fails loud as a decode failure (not a throw, not silent)', async () => {
    const result = await validate(decoder)({ n: 'not-a-number' }).value
    expect(result.isSuccess).toBe(false)
    if (!result.isSuccess) expect(result.reason.kind).toBe('decode')
  })
})
