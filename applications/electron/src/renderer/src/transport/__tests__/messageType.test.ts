import { describe, expect, it } from 'vitest'
import { MESSAGE_TYPE, asMessageType } from '../messageType'

describe('MESSAGE_TYPE source', () => {
  it('mirrors the Rust MessageType wire values exactly', () => {
    expect(Object.values(MESSAGE_TYPE).sort()).toEqual(
      ['activity', 'chat', 'insight', 'nudge', 'state-delta', 'state-snapshot', 'system'].sort()
    )
  })
})

describe('asMessageType boundary decoder', () => {
  it('narrows each known wire string to its typed value', () => {
    for (const wire of Object.values(MESSAGE_TYPE)) {
      expect(asMessageType(wire)).toBe(wire)
    }
  })

  it('returns null for an unknown wire string', () => {
    expect(asMessageType('totally-unknown')).toBeNull()
  })

  it('returns null for a missing discriminator', () => {
    expect(asMessageType(null)).toBeNull()
    expect(asMessageType(undefined)).toBeNull()
  })
})
