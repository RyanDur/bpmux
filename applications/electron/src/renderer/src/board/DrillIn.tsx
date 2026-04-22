import { useCallback, useEffect, useRef, useState } from 'react'
import MessageBody from '../chat/MessageBody'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyTypeFromLabels, type BoardStory } from './types'
import './DrillIn.css'

interface Props {
  story: BoardStory
  isOwner: boolean
  apiBase: string | undefined
  authToken: string | undefined
  usingMock: boolean
  onBack: () => void
  onStoryMutated: (story: BoardStory) => void
}

type ActionMode = 'idle' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

const TOAST_MS = 4000

function DrillIn({
  story,
  isOwner,
  apiBase,
  authToken,
  usingMock,
  onBack,
  onStoryMutated
}: Props): React.JSX.Element {
  const [mode, setMode] = useState<ActionMode>('idle')
  const [busy, setBusy] = useState<BusyKind>(null)
  const [note, setNote] = useState<string>('')
  const [toast, setToast] = useState<string | null>(null)
  const articleRef = useRef<HTMLElement | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)

  // Focus the article on mount so Esc falls to our handler before
  // anything upstream intercepts.
  useEffect(() => {
    articleRef.current?.focus()
  }, [])

  // Autofocus the reject textarea when the reject flow opens so the
  // owner can start typing the note immediately.
  useEffect(() => {
    if (mode === 'rejecting') {
      textareaRef.current?.focus()
    }
  }, [mode])

  useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), TOAST_MS)
    return () => clearTimeout(id)
  }, [toast])

  const canAct = isOwner && (story.state === 'delivered' || story.state === 'awaiting-accept')

  const handleAccept = useCallback(async () => {
    if (busy !== null) return
    setBusy('accept')
    const rollback = (reason: string): void => {
      setBusy(null)
      setToast(`Accept failed: ${reason}`)
    }
    try {
      if (usingMock || !apiBase) {
        // Optimistic local mutation in mock mode so the UX is viewable
        // without a backend.
        onStoryMutated({ ...story, state: 'accepted' })
        setBusy(null)
        onBack()
        return
      }
      const res = await fetch(`${apiBase}/api/stories/${story.number}/accept`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${authToken ?? ''}`,
          'Content-Type': 'application/json'
        }
      })
      if (!res.ok) {
        rollback(`HTTP ${res.status}`)
        return
      }
      onStoryMutated({ ...story, state: 'accepted' })
      setBusy(null)
      onBack()
    } catch (err) {
      rollback(err instanceof Error ? err.message : String(err))
    }
  }, [apiBase, authToken, busy, onBack, onStoryMutated, story, usingMock])

  const handleRejectSend = useCallback(async () => {
    const trimmed = note.trim()
    if (!trimmed || busy !== null) return
    setBusy('reject')
    const rollback = (reason: string): void => {
      setBusy(null)
      setToast(`Reject failed: ${reason}`)
    }
    try {
      if (usingMock || !apiBase) {
        onStoryMutated({ ...story, state: 'in-progress' })
        setBusy(null)
        setMode('idle')
        setNote('')
        onBack()
        return
      }
      const res = await fetch(`${apiBase}/api/stories/${story.number}/reject`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${authToken ?? ''}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ note: trimmed })
      })
      if (!res.ok) {
        rollback(`HTTP ${res.status}`)
        return
      }
      onStoryMutated({ ...story, state: 'in-progress' })
      setBusy(null)
      setMode('idle')
      setNote('')
      onBack()
    } catch (err) {
      rollback(err instanceof Error ? err.message : String(err))
    }
  }, [apiBase, authToken, busy, note, onBack, onStoryMutated, story, usingMock])

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        if (mode === 'rejecting') {
          setMode('idle')
          // preserve note text per spec
          return
        }
        onBack()
      }
    },
    [mode, onBack]
  )

  const handleTextareaKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleRejectSend()
      }
    },
    [handleRejectSend]
  )

  const type = storyTypeFromLabels(story.labels)

  return (
    <article
      className="drill-in"
      ref={articleRef}
      tabIndex={-1}
      data-test="board-drill-in"
      onKeyDown={handleKeyDown}
      aria-label={`Story ${story.number}: ${story.title}`}
    >
      <header className="drill-in-header">
        <button
          type="button"
          className="drill-in-back"
          onClick={onBack}
          aria-label="Back to board list"
        >
          <BackArrow />
          <small className="drill-in-crumb">Board · #{story.number}</small>
        </button>
      </header>

      <section className="drill-in-meta">
        <h3 className="drill-in-title">
          <StoryTypeGlyph type={type} />
          <b className="drill-in-title-text">{story.title}</b>
        </h3>
        <dl className="drill-in-meta-list">
          <dt>state</dt>
          <dd>{story.state}</dd>
          <dt>labels</dt>
          <dd>
            {story.labels.length === 0 ? (
              <em>(none)</em>
            ) : (
              <ul className="drill-in-chips">
                {story.labels.map((label) => (
                  <li key={label} className="drill-in-chip">
                    {label}
                  </li>
                ))}
              </ul>
            )}
          </dd>
          <dt>assignees</dt>
          <dd>
            {story.assignees.length === 0 ? (
              <em>(none)</em>
            ) : (
              <ul className="drill-in-chips">
                {story.assignees.map((a) => (
                  <li key={a} className="drill-in-chip">
                    {a}
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </dl>
      </section>

      <section className="drill-in-body" aria-label="Story body">
        {story.body ? (
          <MessageBody text={story.body} />
        ) : (
          <p className="drill-in-body-empty">(no body)</p>
        )}
      </section>

      {canAct ? (
        <footer className="drill-in-actions">
          {mode === 'idle' ? (
            <menu className="drill-in-action-row" data-test="drill-in-actions">
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--primary"
                  disabled={busy !== null}
                  onClick={() => {
                    void handleAccept()
                  }}
                  data-test="drill-in-accept"
                >
                  {busy === 'accept' ? 'Accepting…' : 'Accept'}
                </button>
              </li>
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--secondary"
                  disabled={busy !== null}
                  onClick={() => setMode('rejecting')}
                  data-test="drill-in-reject"
                >
                  Reject
                </button>
              </li>
            </menu>
          ) : (
            <form
              className="drill-in-reject-form"
              onSubmit={(event) => {
                event.preventDefault()
                void handleRejectSend()
              }}
            >
              <label className="drill-in-reject-label">
                <small>Rejection note (required)</small>
                <textarea
                  ref={textareaRef}
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  onKeyDown={handleTextareaKeyDown}
                  placeholder="What needs rework?"
                  rows={3}
                  data-test="drill-in-reject-note"
                />
              </label>
              <menu className="drill-in-action-row">
                <li>
                  <button
                    type="submit"
                    className="drill-in-btn drill-in-btn--primary"
                    disabled={busy !== null || note.trim().length === 0}
                    data-test="drill-in-reject-send"
                  >
                    {busy === 'reject' ? 'Sending…' : 'Send'}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    className="drill-in-btn drill-in-btn--ghost"
                    disabled={busy !== null}
                    onClick={() => setMode('idle')}
                  >
                    Cancel
                  </button>
                </li>
              </menu>
            </form>
          )}
        </footer>
      ) : null}

      {toast ? (
        <aside className="drill-in-toast" role="status" data-test="drill-in-toast">
          {toast}
        </aside>
      ) : null}
    </article>
  )
}

function BackArrow(): React.JSX.Element {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      aria-hidden="true"
      focusable="false"
      className="drill-in-back-arrow"
    >
      <path
        d="M7.5 2L3 6l4.5 4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

export default DrillIn
