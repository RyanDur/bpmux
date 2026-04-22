import { format } from 'date-fns'
import { useEffect, useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
}

function formatTime(ts: string): string {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

// Strip the `:tool` suffix the server adds to identities so the UI
// shows `dev` rather than `dev:claude-code`. Pure mirror of
// `ws.rs::short_name`.
function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function MessageList({ messages }: Props): React.JSX.Element {
  const endRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    // Scroll-to-bottom on every new message. `auto` rather than
    // `smooth` so a fast-streaming agent burst doesn't lag the
    // scroll position behind the latest frame.
    endRef.current?.scrollIntoView({ block: 'end' })
  }, [messages.length])

  if (messages.length === 0) {
    return (
      <div className="message-list message-list--empty" data-test="message-list">
        <div className="message-empty">no messages yet</div>
      </div>
    )
  }

  const visible = messages.filter(
    (m) => m.messageType !== 'activity' && m.messageType !== 'system'
  )

  if (visible.length === 0) {
    return (
      <div className="message-list message-list--empty" data-test="message-list">
        <div className="message-empty">no messages yet</div>
        <div ref={endRef} />
      </div>
    )
  }

  return (
    <div className="message-list" data-test="message-list">
      {visible.map((m) => {
        const scope = scopeLabel(m)
        return (
          <div key={m.id} className="message-row" data-test="message-row">
            <div className="message-meta">
              <span className="message-from">{shortName(m.from)}</span>
              <span className="message-ts">{formatTime(m.ts)}</span>
              {scope && <span className="message-scope">{scope}</span>}
            </div>
            <div className="message-body">{m.text}</div>
          </div>
        )
      })}
      <div ref={endRef} />
    </div>
  )
}

function scopeLabel(m: ChatMessage): string | null {
  if (!m.to || m.to.length === 0) return null
  if (m.to.includes('all')) return null
  return `→ ${m.to.join(', ')}`
}

export default MessageList
