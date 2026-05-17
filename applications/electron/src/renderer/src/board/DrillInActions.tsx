import { useCallback, useEffect, useRef, useState } from 'react'
import { describeHttpError } from '../api/http'
import { postStoryAccept, postStoryReject } from '../api/storiesClient'
import type { BoardStory } from './types'

export type ActionMode = 'idle' | 'bug-question' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

interface Props {
  story: BoardStory
  canAct: boolean
  apiBase: string | undefined
  authToken: string | undefined
  usingMock: boolean
  mode: ActionMode
  setMode: (m: ActionMode) => void
  onStoryMutated: (story: BoardStory) => void
  onBack: () => void
  onError: (msg: string) => void
}

const DrillInActions = ({
  story,
  canAct,
  apiBase,
  authToken,
  usingMock,
  mode,
  setMode,
  onStoryMutated,
  onBack,
  onError
}: Props): React.JSX.Element | null => {
  const [busy, setBusy] = useState<BusyKind>(null)
  const [note, setNote] = useState<string>('')
  const [bypassReason, setBypassReason] = useState<string>('')
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const rejectDialogRef = useRef<HTMLDialogElement | null>(null)

  useEffect(() => {
    const dialog = rejectDialogRef.current
    if (!dialog) return
    if (mode === 'rejecting') {
      if (!dialog.open) dialog.showModal()
      textareaRef.current?.focus()
    } else if (dialog.open) {
      dialog.close()
    }
  }, [mode])

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
        onError(`Accept failed: ${reason}`)
      }
      if (usingMock || apiBase === undefined) {
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
    [apiBase, authToken, busy, onBack, onError, onStoryMutated, story, usingMock]
  )

  const handleAccept = useCallback((): void => {
    setMode('bug-question')
  }, [setMode])

  const handleAcceptConfirm = useCallback((): void => {
    setMode('idle')
    void fireAccept({ would_file_bug: false })
  }, [fireAccept, setMode])

  const handleAcceptRouteToReject = useCallback((): void => {
    setMode('rejecting')
  }, [setMode])

  const handleAcceptBypass = useCallback((): void => {
    const trimmed = bypassReason.trim()
    if (!trimmed) return
    setMode('idle')
    setBypassReason('')
    void fireAccept({ would_file_bug: true, force: true, bypass_reason: trimmed })
  }, [bypassReason, fireAccept, setMode])

  const handleRejectSend = useCallback(async () => {
    const trimmed = note.trim()
    if (!trimmed || busy !== null) return
    setBusy('reject')
    const rollback = (reason: string): void => {
      setBusy(null)
      onError(`Reject failed: ${reason}`)
    }
    if (usingMock || apiBase === undefined) {
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
  }, [apiBase, authToken, busy, note, onBack, onError, onStoryMutated, setMode, story, usingMock])

  const handleTextareaKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleRejectSend()
      }
    },
    [handleRejectSend]
  )

  if (!canAct) return null

  return (
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
  )
}

export default DrillInActions
