import { useCallback, useEffect, useRef, useState } from 'react'
import { describeHttpError } from '@transport/http'
import { postStoryAccept, postStoryReject } from '@transport/storiesClient'
import BugQuestionDialog from './BugQuestionDialog'
import type { Story } from '../types'

export type ActionMode = 'idle' | 'bug-question' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

type Props = {
  story: Story
  canAct: boolean
  apiBase: string | undefined
  authToken: string | undefined
  ownerNonce?: string | undefined
  mode: ActionMode
  setMode: (m: ActionMode) => void
  onStoryMutated: (story: Story) => void
  onBack: () => void
  onError: (msg: string) => void
}

const DrillInActions = ({
  story,
  canAct,
  apiBase,
  authToken,
  ownerNonce,
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
      if (apiBase === undefined) {
        onStoryMutated({ ...story, state: 'accepted' })
        setBusy(null)
        onBack()
        return
      }
      const proceed = (
        await postStoryAccept({ apiBase, authToken, ownerNonce }, story.id, payload).value
      ).either(
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
    [apiBase, authToken, ownerNonce, busy, onBack, onError, onStoryMutated, story]
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
    if (apiBase === undefined) {
      onStoryMutated({ ...story, state: 'ready' })
      setBusy(null)
      setMode('idle')
      setNote('')
      onBack()
      return
    }
    const proceed = (
      await postStoryReject({ apiBase, authToken, ownerNonce }, story.id, { note: trimmed }).value
    ).either(
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
  }, [apiBase, authToken, ownerNonce, busy, note, onBack, onError, onStoryMutated, setMode, story])

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
        aria-labelledby="drill-in-reject-title"
        onClose={() => {
          if (mode === 'rejecting') setMode('idle')
        }}
      >
        <form
          className="drill-in-reject-form-body"
          onSubmit={(event) => {
            event.preventDefault()
            void handleRejectSend()
          }}
        >
          <h2 id="drill-in-reject-title" className="drill-in-reject-title">
            Reject story
          </h2>
          <label className="drill-in-reject-label">
            <small className="drill-in-reject-field-label">Reason (sent to the team)</small>
            <textarea
              ref={textareaRef}
              className="drill-in-reject-textarea"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              onKeyDown={handleTextareaKeyDown}
              placeholder="What needs rework?"
              rows={3}
              data-test="drill-in-reject-note"
            />
          </label>
          <menu className="drill-in-action-row drill-in-reject-actions">
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
          </menu>
        </form>
      </dialog>
      <BugQuestionDialog
        open={mode === 'bug-question'}
        busy={busy !== null}
        bypassReason={bypassReason}
        setBypassReason={setBypassReason}
        onAcceptConfirm={handleAcceptConfirm}
        onAcceptRouteToReject={handleAcceptRouteToReject}
        onAcceptBypass={handleAcceptBypass}
      />
    </footer>
  )
}

export default DrillInActions
