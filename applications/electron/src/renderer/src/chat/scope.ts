import type { ChatMessage } from '../transport/types'

const shortNameOf = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

const computeMessageChannelId = (m: ChatMessage): string => {
  const to = m.to
  if (!to || to.length === 0 || to.includes('all')) return 'broadcast'
  const sender = shortNameOf(m.from)
  const participants = new Set<string>([sender, ...to])
  return [...participants].sort().join('+')
}

const computeViewChannelId = (recipient: string[] | null, viewer: string): string => {
  if (!recipient || recipient.length === 0) return 'broadcast'
  return [viewer, ...recipient].sort().join('+')
}

export const isMessageInScope = (
  m: ChatMessage,
  recipient: string[] | null | undefined,
  viewer: string | undefined,
  allView: boolean = false
): boolean => {
  if (allView) return true
  const v = viewer ?? ''
  const r = recipient ?? null
  const to = m.to
  const isBroadcast = !to || to.length === 0 || to.includes('all')
  if (isBroadcast && (r === null || r.length === 0)) return true
  return computeMessageChannelId(m) === computeViewChannelId(r, v)
}
