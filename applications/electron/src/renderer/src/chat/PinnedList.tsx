import { format } from 'date-fns'
import { useCallback } from 'react'
import type { Pin } from '../hooks/usePins'
import MessageBody from './MessageBody'
import './PinnedList.css'

interface Props {
  pins: Pin[]
  onUnpin: (messageKey: string) => void
  onClose: () => void
}

function shortName(identity: string): string {
  const idx = identity.indexOf(':')
  return idx === -1 ? identity : identity.slice(0, idx)
}

function PinnedList({ pins, onUnpin, onClose }: Props): React.JSX.Element {
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>): void => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
      }
    },
    [onClose]
  )

  return (
    <section
      className="pinned-list"
      data-test="pinned-list-overlay"
      aria-label="Pinned messages"
      onKeyDown={handleKeyDown}
      tabIndex={-1}
    >
      <header className="pinned-list-header">
        <h2 className="pinned-list-title">
          Pinned <span className="pinned-list-count">{pins.length}</span>
        </h2>
        <button
          type="button"
          className="pinned-list-close"
          onClick={onClose}
          aria-label="Close pinned list"
        >
          ✕
        </button>
      </header>
      {pins.length === 0 ? (
        <p className="pinned-list-empty">no pinned messages in this channel</p>
      ) : (
        <ul className="pinned-list-items" data-test="pinned-list-items">
          {pins.map((p) => (
            <li key={p.message_key} className="pinned-item" data-test={`pinned-item-${p.message_key}`}>
              <article>
                <header className="pinned-item-meta">
                  <cite className="pinned-item-from">{shortName(p.from)}</cite>
                  <time dateTime={p.ts}>
                    {format(new Date(p.ts), 'MMM d · h:mm a zzz')}
                  </time>
                  <small className="pinned-item-by">
                    pinned by {p.pinned_by}
                  </small>
                </header>
                <section className="pinned-item-body">
                  <MessageBody text={p.text} />
                </section>
                <footer className="pinned-item-actions">
                  <button
                    type="button"
                    className="pinned-item-unpin"
                    data-test={`pinned-item-unpin-${p.message_key}`}
                    onClick={() => onUnpin(p.message_key)}
                  >
                    Unpin
                  </button>
                </footer>
              </article>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

export default PinnedList
