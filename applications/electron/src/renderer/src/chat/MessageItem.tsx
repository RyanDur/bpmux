import { format } from 'date-fns'
import type { ChatMessage } from '../transport/types'
import { classOf } from '../hooks/useMessageFilter'
import type { AggregatedReaction } from '../hooks/useReactions'
import { roleColorFor } from '../roles/roleColor'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import ClassGlyph from './ClassGlyph'
import MessageBody from './MessageBody'
import Reactions, { ReactionAddButton } from './Reactions'
import UnfurlCards from './UnfurlCards'
import { channelLabelOf } from './scope'

interface Props {
  m: ChatMessage
  grouped: boolean
  viewer?: string
  apiBase?: string
  authToken?: string
  presence: Map<string, string>
  mentionRoles: Set<string>
  onMention: (role: string) => void
  reactions?: AggregatedReaction[]
  onToggleReaction?: (messageKey: string, emoji: string) => void
}

const formatTime = (ts: string): string => {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

const shortName = (identity: string): string => {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

const avatarClasses = (sender: string): string => {
  const shape = sender === 'ryan' ? 'message-avatar--circle' : 'message-avatar--square'
  return `message-avatar ${shape}`
}

const messageKey = (m: ChatMessage): string => `${m.ts}|${m.from}`

const MessageItem = ({
  m,
  grouped,
  viewer,
  apiBase,
  authToken,
  presence,
  mentionRoles,
  onMention,
  reactions,
  onToggleReaction
}: Props): React.JSX.Element => {
  const sender = shortName(m.from)
  const channel = channelLabelOf(m, viewer)
  const addressee = m.addressee ?? null
  const addressedToViewer = addressee === RENDERER_IDENTITY
  const addressedToOther = addressee !== null && !addressedToViewer

  const rowClasses = [
    'message-row',
    grouped ? 'message-row--grouped' : 'message-row--first',
    addressedToViewer ? 'message-row--to-me' : '',
    addressedToOther ? 'message-row--to-other' : ''
  ]
    .filter(Boolean)
    .join(' ')

  const mKey = messageKey(m)

  return (
    <li key={m.id} className={rowClasses} data-test="message-row">
      <article>
        <span className="sr-only">{sender}:</span>
        {!grouped && (() => {
          const label = sender.charAt(0).toUpperCase() + sender.slice(1)
          const status = presence.get(sender) ?? 'offline'
          const tooltipId = `avatar-info-${m.id}`
          const roleColor = roleColorFor(sender)
          return (
            <span className="message-avatar-wrap">
              <button
                type="button"
                className={avatarClasses(sender)}
                data-test="message-avatar"
                data-role={sender}
                style={{
                  background: roleColor.background,
                  color: roleColor.color
                }}
                aria-describedby={tooltipId}
                aria-label={`${label} — ${status}`}
              >
                {sender.charAt(0).toUpperCase()}
              </button>
              <span
                id={tooltipId}
                role="tooltip"
                className="avatar-tooltip"
                data-test="avatar-tooltip"
              >
                <strong className="avatar-tooltip-label">{label}</strong>
                <small
                  className={`avatar-tooltip-status avatar-tooltip-status--${status}`}
                  data-test="avatar-tooltip-status"
                >
                  {status}
                </small>
              </span>
            </span>
          )
        })()}
        {!grouped && (
          <header className="message-meta">
            <cite className="message-from" aria-hidden="true">
              {sender}
            </cite>
            <time className="message-ts" dateTime={m.ts}>
              {formatTime(m.ts)}
            </time>
            <small className="message-channel" data-test="message-channel">
              [{channel}]
            </small>
            {addressee && (
              <small
                className="message-addressee"
                title={`attention: @${addressee}`}
              >
                @{addressee}
              </small>
            )}
          </header>
        )}
        {grouped && (
          <time className="message-ts message-ts--grouped" dateTime={m.ts}>
            {formatTime(m.ts)}
          </time>
        )}
        <section className="message-body">
          <ClassGlyph cls={classOf(m.messageType)} />
          <MessageBody
            text={m.text}
            knownRoles={mentionRoles}
            onMention={onMention}
          />
          <UnfurlCards text={m.text} apiBase={apiBase} authToken={authToken} />
          {onToggleReaction && viewer && (() => {
            if (reactions && reactions.length > 0) {
              return (
                <Reactions
                  reactions={reactions}
                  viewer={viewer}
                  onToggle={(emoji) => onToggleReaction(mKey, emoji)}
                />
              )
            }
            return (
              <ReactionAddButton
                onToggle={(emoji) => onToggleReaction(mKey, emoji)}
              />
            )
          })()}
        </section>
      </article>
    </li>
  )
}

export default MessageItem
export { messageKey, shortName, formatTime, avatarClasses }
