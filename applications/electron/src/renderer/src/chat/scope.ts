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
  if (isBroadcast || r === null || r.length === 0) return false
  // #463: group views (recipient ≥ 2) include agent-only conversations
  // among the group — Ryan-as-viewer wants to observe team coordination
  // even when he wasn't a participant. Strict equality (the pre-#463
  // semantic) hid those conversations because their channelId omits
  // viewer. New rule: every group recipient must appear in the
  // message's participants AND no participant outside {viewer, group}.
  // DM (recipient = 1) keeps the strict equality semantic per #429.
  const senderShort = shortNameOf(m.from)
  const participants = new Set<string>([senderShort, ...(to ?? [])])
  if (r.length >= 2) {
    const allowed = new Set<string>([shortNameOf(v), ...r])
    const withinAllowed = [...participants].every((p) => allowed.has(p))
    const groupFullyPresent = r.every((rcp) => participants.has(rcp))
    return withinAllowed && groupFullyPresent
  }
  return computeMessageChannelId(m) === computeViewChannelId(r, v)
}

export const channelLabelOf = (m: ChatMessage, viewer: string | undefined): string => {
  const to = m.to
  if (!to || to.length === 0 || to.includes('all')) return 'broadcast'
  const v = shortNameOf(viewer ?? '')
  const sender = shortNameOf(m.from)
  const participants = new Set<string>([sender, ...to])
  participants.delete(v)
  return [...participants].sort().join('+')
}
