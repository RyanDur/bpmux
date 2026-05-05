import { describe, expect, it } from 'vitest'
import type { ChatMessage } from '../transport/types'
import { SYNC_STATUS_UPDATE_MARKER } from '../transport/types'
import { newMessagesContainMarker } from './markerDetection'

const msg = (text: string, id: string): ChatMessage => ({
  id,
  from: 'bpmux',
  text,
  ts: '2026-05-05T00:00:00Z'
})

describe('#1164 SyncStatusGlyph WS-marker push', () => {
  it('detects SYNC_STATUS_UPDATE_MARKER when it arrives in the new-messages window', () => {
    const messages: ChatMessage[] = [
      msg('seed', 'm1'),
      msg('seed', 'm2'),
      msg(SYNC_STATUS_UPDATE_MARKER, 'm3')
    ]
    expect(newMessagesContainMarker(2, messages, SYNC_STATUS_UPDATE_MARKER)).toBe(true)
  })

  it('ignores hydration backfill — marker present but already seen returns false', () => {
    const messages: ChatMessage[] = [
      msg(SYNC_STATUS_UPDATE_MARKER, 'm1'),
      msg('chat', 'm2')
    ]
    expect(newMessagesContainMarker(2, messages, SYNC_STATUS_UPDATE_MARKER)).toBe(false)
  })

  it('returns false when no new messages contain the marker', () => {
    const messages: ChatMessage[] = [
      msg('seed', 'm1'),
      msg('seed', 'm2'),
      msg('plain chat', 'm3'),
      msg('another chat', 'm4')
    ]
    expect(newMessagesContainMarker(2, messages, SYNC_STATUS_UPDATE_MARKER)).toBe(false)
  })

  it('detects the marker even when interleaved with non-marker messages in the new window', () => {
    const messages: ChatMessage[] = [
      msg('seed', 'm1'),
      msg('chat 1', 'm2'),
      msg(SYNC_STATUS_UPDATE_MARKER, 'm3'),
      msg('chat 2', 'm4')
    ]
    expect(newMessagesContainMarker(1, messages, SYNC_STATUS_UPDATE_MARKER)).toBe(true)
  })

  it('returns false on empty messages list', () => {
    expect(newMessagesContainMarker(0, [], SYNC_STATUS_UPDATE_MARKER)).toBe(false)
  })

  it('does not match the marker via substring — exact text only', () => {
    const messages: ChatMessage[] = [
      msg(`prefix ${SYNC_STATUS_UPDATE_MARKER} suffix`, 'm1')
    ]
    expect(newMessagesContainMarker(0, messages, SYNC_STATUS_UPDATE_MARKER)).toBe(false)
  })
})
