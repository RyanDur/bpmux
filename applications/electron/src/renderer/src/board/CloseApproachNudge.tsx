import './CloseApproachNudge.css'

type CloseApproachNudgeProps = {
  currentIterationName: string
  nextIterationName: string
  activeGapLabel: string
  dismissed: boolean
  onViewRoadmap: () => void
  onDismiss: () => void
}

const CloseApproachNudge = ({
  currentIterationName,
  nextIterationName,
  activeGapLabel,
  dismissed,
  onViewRoadmap,
  onDismiss
}: CloseApproachNudgeProps): React.JSX.Element => (
  <aside
    className="close-approach-nudge"
    data-test="close-approach-nudge"
    data-dismissed={dismissed ? 'true' : undefined}
    aria-live="polite"
    aria-label="Next iteration readiness"
  >
    <i className="close-approach-nudge-cue" aria-hidden="true">
      ◐
    </i>
    <p className="close-approach-nudge-msg">
      <strong>{currentIterationName}</strong> closes soon — the next iteration{' '}
      <strong>{nextIterationName}</strong> isn&apos;t ready ({activeGapLabel} open).
    </p>
    <button
      type="button"
      className="close-approach-nudge-link"
      data-test="close-approach-nudge-view"
      onClick={onViewRoadmap}
    >
      View roadmap →
    </button>
    <button
      type="button"
      className="close-approach-nudge-dismiss"
      data-test="close-approach-nudge-dismiss"
      onClick={onDismiss}
      aria-label="Dismiss"
    >
      ×
    </button>
  </aside>
)

export default CloseApproachNudge
