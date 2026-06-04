export const MESSAGE_TYPE = {
  Chat: 'chat',
  Activity: 'activity',
  System: 'system',
  Insight: 'insight',
  Nudge: 'nudge',
  StateDelta: 'state-delta',
  StateSnapshot: 'state-snapshot'
} as const

export type MessageType = (typeof MESSAGE_TYPE)[keyof typeof MESSAGE_TYPE]

const MESSAGE_TYPE_VALUES: readonly string[] = Object.values(MESSAGE_TYPE)

export const asMessageType = (raw: string | null | undefined): MessageType | null =>
  raw != null && MESSAGE_TYPE_VALUES.includes(raw) ? (raw as MessageType) : null
