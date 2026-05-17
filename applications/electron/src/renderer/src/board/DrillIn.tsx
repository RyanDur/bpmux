import { useCallback, useEffect, useRef, useState } from 'react'
import MessageBody from '../chat/MessageBody'
import { describeHttpError } from '../api/http'
import { postStoryAccept, postStoryReject } from '../api/storiesClient'
import type { ChatMessage } from '../transport/types'
import Comments from './Comments'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import Timeline from './Timeline'
import { storyTypeFromLabels, type BoardStory } from './types'
import './DrillIn.css'

const LIFECYCLE_LABELS: ReadonlySet<string> = new Set([
  'delivered',
  'ready for accept',
  'ready-for-accept',
  'desk-check-fail',
  'desk-check-pass',
  'blocked',
  'rework',
  'rejected',
  'ready'
])

const isLifecycleLabel = (label: string): boolean =>
  LIFECYCLE_LABELS.has(label) || label.startsWith('blocked-by:')

interface Props {
  story: BoardStory
  isOwner: boolean
  apiBase: string | undefined
  authToken: string | undefined
  usingMock: boolean
  // Chat message stream from the WS. DrillIn watches this for
  // BOARD_INVALIDATE_MARKER messages (published when any peer posts
  // a comment, accepts/rejects, etc.) and refetches comments live —
  // the viewer sees peer activity without having to exit + re-drill.
  messages: ChatMessage[]
  // #354 close-direction: Board's useAnimatedUnmount flips this to
  // false to trigger the exit transition before unmount.
  visible: boolean
  // #368: when true, DrillIn auto-expands the timeline section + scrolls
  // it into view on mount. Used by the team-accept badge click path so
  // opening a drill-in via the badge lands directly on the audit trail.
  focusTimeline?: boolean
  onBack: () => void
  onStoryMutated: (story: BoardStory) => void
}

type ActionMode = 'idle' | 'bug-question' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

const TOAST_MS = 4000

const DrillIn = ({
  story,
  isOwner,
  apiBase,
  authToken,
  usingMock,
  messages,
  visible,
  focusTimeline,
  onBack,
  onStoryMutated
}: Props): React.JSX.Element => {
  const [mode, setMode] = useState<ActionMode>('idle')
  const [busy, setBusy] = useState<BusyKind>(null)
  const [note, setNote] = useState<string>('')
  const [bypassReason, setBypassReason] = useState<string>('')
  const [toast, setToast] = useState<string | null>(null)
  const articleRef = useRef<HTMLElement | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const rejectDialogRef = useRef<HTMLDialogElement | null>(null)


  // Focus the article on mount so Esc falls to our handler before
  // anything upstream intercepts.
  useEffect(() => {
    articleRef.current?.focus()
  }, [])

  useEffect(() => {
    const dialog = rejectDialogRef.current
    if (!dialog) return
    if (mode === 'rejecting') {
      if (!dialog.open) {
        dialog.showModal()
      }
      textareaRef.current?.focus()
    } else if (dialog.open) {
      dialog.close()
    }
  }, [mode])

useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), TOAST_MS)
    return () => clearTimeout(id)
  }, [toast])

  const canAct = isOwner && story.state === 'delivered'

  const fireAccept = useCallback(
    async (payload: {
      would_file_bug: boolean
      force?: boolean
      bypass_reason?: string
    }): Promise<void> => {
      if (busy !== null) return
      setBusy('accept')
      const rollback = (reason: string): void => {
        setBusy(null)
        setToast(`Accept failed: ${reason}`)
      }
      if (usingMock || !apiBase) {
        onStoryMutated({ ...story, state: 'accepted' })
        setBusy(null)
        onBack()
        return
      }
      const proceed = await postStoryAccept({ apiBase, authToken }, story.number, payload).mapEither(
        () => true,
        (error) => {
          rollback(describeHttpError(error))
          return false
        }
      )
      if (!proceed) return
      onStoryMutated({ ...story, state: 'accepted' })
      setBusy(null)
      onBack()
    },
    [apiBase, authToken, busy, onBack, onStoryMutated, story, usingMock]
  )

  const handleAccept = useCallback((): void => {
    setMode('bug-question')
  }, [])

  const handleAcceptConfirm = useCallback((): void => {
    setMode('idle')
    void fireAccept({ would_file_bug: false })
  }, [fireAccept])

  const handleAcceptRouteToReject = useCallback((): void => {
    setMode('rejecting')
  }, [])

  const handleAcceptBypass = useCallback((): void => {
    const trimmed = bypassReason.trim()
    if (!trimmed) return
    setMode('idle')
    setBypassReason('')
    void fireAccept({ would_file_bug: true, force: true, bypass_reason: trimmed })
  }, [bypassReason, fireAccept])

  const handleRejectSend = useCallback(async () => {
    const trimmed = note.trim()
    if (!trimmed || busy !== null) return
    setBusy('reject')
    const rollback = (reason: string): void => {
      setBusy(null)
      setToast(`Reject failed: ${reason}`)
    }
    if (usingMock || !apiBase) {
      onStoryMutated({ ...story, state: 'ready' })
      setBusy(null)
      setMode('idle')
      setNote('')
      onBack()
      return
    }
    const proceed = await postStoryReject(
      { apiBase, authToken },
      story.number,
      { note: trimmed }
    ).mapEither(
      () => true,
      (error) => {
        rollback(describeHttpError(error))
        return false
      }
    )
    if (!proceed) return
    onStoryMutated({ ...story, state: 'ready' })
    setBusy(null)
    setMode('idle')
    setNote('')
    onBack()
  }, [apiBase, authToken, busy, note, onBack, onStoryMutated, story, usingMock])

  // #366 quote-excerpt: shared insertion logic. Either the keyboard
  // shortcut (Cmd/Ctrl+E) or the native right-click "Quote in reply"
  // context-menu item (#371) calls this with the selection's
  // comment id + author + text. Prepends a markdown blockquote to
  // the comment draft, then focuses the textarea with the cursor
  // placed AFTER the blank line that follows the quote block so
  // the author can type their reply immediately. The anchor in the
  // blockquote links back to the source comment via `#comment-<id>`;
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (event.defaultPrevented) return
      if (event.key === 'Escape') {
        if (mode === 'rejecting') return
        event.preventDefault()
        if (mode === 'bug-question') {
          setMode('idle')
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
      data-visible={visible || undefined}
      aria-hidden={!visible || undefined}
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
        <StoryBadge story={story} />
        {(() => {
          const meaningfulLabels = story.labels.filter((label) => !isLifecycleLabel(label))
          if (meaningfulLabels.length === 0) return null
          return (
            <ul className="drill-in-chips" data-test="drill-in-labels">
              {meaningfulLabels.map((label) => (
                <li key={label} className="drill-in-chip">
                  {label}
                </li>
              ))}
            </ul>
          )
        })()}
        {story.assignees.length > 0 && (
          <ul className="drill-in-chips" data-test="drill-in-assignees">
            {story.assignees.map((a) => (
              <li key={a} className="drill-in-chip">
                {a}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="drill-in-body" aria-label="Story body">
        {story.body ? (
          <MessageBody text={story.body} />
        ) : (
          <p className="drill-in-body-empty">(no body)</p>
        )}
      </section>

      <Comments
        storyNumber={story.number}
        apiBase={apiBase}
        authToken={authToken}
        usingMock={usingMock}
        messages={messages}
        onError={setToast}
      />

      <Timeline
        story={story}
        apiBase={apiBase}
        authToken={authToken}
        messages={messages}
        focusTimeline={focusTimeline}
      />

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
          ) : null}
          <dialog
            ref={rejectDialogRef}
            className="drill-in-reject-form"
            aria-label="Reject story"
            onClose={() => {
              if (mode === 'rejecting') setMode('idle')
            }}
          >
            <form
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
          </dialog>
          <dialog
            open={mode === 'bug-question'}
            className="drill-in-bug-question"
            aria-label="Bug-question test"
            data-test="drill-in-bug-question"
          >
            <h3>Would you file a bug about this story if you accepted it as-is?</h3>
            <p>
              The bug-question test catches accept-and-file-bug-as-deferral. If you would file
              a bug, the story still has work — use Reject instead.
            </p>
            <menu className="drill-in-action-row">
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--primary"
                  disabled={busy !== null}
                  onClick={handleAcceptConfirm}
                  data-test="drill-in-bug-question-no"
                >
                  No, accept it
                </button>
              </li>
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--secondary"
                  disabled={busy !== null}
                  onClick={handleAcceptRouteToReject}
                  data-test="drill-in-bug-question-yes"
                >
                  Yes, reject instead
                </button>
              </li>
            </menu>
            <details className="drill-in-bypass">
              <summary>Override bug-question (require justification)</summary>
              <form
                onSubmit={(event) => {
                  event.preventDefault()
                  void handleAcceptBypass()
                }}
              >
                <label className="drill-in-reject-label">
                  <small>Bypass reason (required, recorded in audit trail)</small>
                  <textarea
                    value={bypassReason}
                    onChange={(e) => setBypassReason(e.target.value)}
                    placeholder="Why is this bypass justified?"
                    rows={3}
                    data-test="drill-in-bypass-reason"
                  />
                </label>
                <menu className="drill-in-action-row">
                  <li>
                    <button
                      type="submit"
                      className="drill-in-btn drill-in-btn--secondary"
                      disabled={busy !== null || bypassReason.trim().length === 0}
                      data-test="drill-in-bypass-submit"
                    >
                      Force accept
                    </button>
                  </li>
                </menu>
              </form>
            </details>
          </dialog>
        </footer>
      ) : null}

      {toast ? (
        <output className="drill-in-toast" aria-live="polite" data-test="drill-in-toast">
          {toast}
        </output>
      ) : null}
    </article>
  )
}

const BackArrow = (): React.JSX.Element => (
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

export default DrillIn
