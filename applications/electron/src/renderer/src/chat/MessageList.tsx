import { format } from 'date-fns'
import { useEffect, useRef } from 'react'
import type { ChatMessage } from '../transport/types'
import MessageBody from './MessageBody'
import './MessageList.css'

interface Props {
  messages: ChatMessage[]
}

function formatTime(ts: string): string {
  const d = new Date(ts)
  if (Number.isNaN(d.getTime())) return ts
  return format(d, 'h:mm a zzz')
}

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function scopeLabel(m: ChatMessage): string | null {
  if (!m.to || m.to.length === 0) return null
  if (m.to.includes('all')) return null
  return `→ ${m.to.join(', ')}`
}

function MessageList({ messages }: Props): React.JSX.Element {
  const endRef = useRef<HTMLLIElement>(null)

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: 'end' })
  }, [messages.length])

  const visible = messages.filter(
    (m) => m.messageType !== 'activity' && m.messageType !== 'system'
  )

  if (visible.length === 0) {
    return (
      <ul className="message-list message-list--empty" data-test="message-list">
        <li className="message-empty">no messages yet</li>
        <li ref={endRef} aria-hidden="true" />
      </ul>
    )
  }

  return (
    <ul className="message-list" data-test="message-list">
      {visible.map((m) => {
        const scope = scopeLabel(m)
        const addressee = m.addressee ?? null
        return (
          <li key={m.id} className="message-row" data-test="message-row">
            <article>
              <header className="message-meta">
                <cite className="message-from">{shortName(m.from)}</cite>
                <time className="message-ts" dateTime={m.ts}>
                  {formatTime(m.ts)}
                </time>
                {scope && <small className="message-scope">{scope}</small>}
                {addressee && (
                  <small className="message-addressee" title={`attention: @${addressee}`}>
                    @{addressee}
                  </small>
                )}
              </header>
              <section className="message-body">
                <MessageBody text={m.text} />
              </section>
            </article>
          </li>
        )
      })}
      <li ref={endRef} aria-hidden="true" />
    </ul>
  )
}

export default MessageList
