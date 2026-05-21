interface Props {
  open: boolean
  busy: boolean
  bypassReason: string
  setBypassReason: (s: string) => void
  onAcceptConfirm: () => void
  onAcceptRouteToReject: () => void
  onAcceptBypass: () => void
}

const BugQuestionDialog = ({
  open,
  busy,
  bypassReason,
  setBypassReason,
  onAcceptConfirm,
  onAcceptRouteToReject,
  onAcceptBypass
}: Props): React.JSX.Element => (
  <dialog
    open={open}
    className="drill-in-bug-question"
    aria-label="Bug-question test"
    data-test="drill-in-bug-question"
  >
    <h3>Would you file a bug about this story if you accepted it as-is?</h3>
    <p>
      The bug-question test catches accept-and-file-bug-as-deferral. If you would file a bug, the
      story still has work — use Reject instead.
    </p>
    <menu className="drill-in-action-row">
      <li>
        <button
          type="button"
          className="drill-in-btn drill-in-btn--primary"
          disabled={busy}
          onClick={onAcceptConfirm}
          data-test="drill-in-bug-question-no"
        >
          No, accept it
        </button>
      </li>
      <li>
        <button
          type="button"
          className="drill-in-btn drill-in-btn--secondary"
          disabled={busy}
          onClick={onAcceptRouteToReject}
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
          onAcceptBypass()
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
              disabled={busy || bypassReason.trim().length === 0}
              data-test="drill-in-bypass-submit"
            >
              Force accept
            </button>
          </li>
        </menu>
      </form>
    </details>
  </dialog>
)

export default BugQuestionDialog
