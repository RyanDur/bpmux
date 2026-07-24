import { useEffect, useRef, useState } from 'react'
import MessageBody from '../../chat/MessageBody'
import Comments from './Comments'
import DrillInActions, { type ActionMode } from './DrillInActions'
import StoryBadge from '../stories/StoryBadge'
import StoryTypeGlyph from '../stories/StoryTypeGlyph'
import Timeline from './Timeline'
import { storyTypeFromLabels, type Story } from '../types'
import './DrillIn.css'

const LIFECYCLE_LABELS: ReadonlySet<string> = new Set([
  'delivered',
  'ready for accept',
  'ready-for-accept',
  'desk-check-fail',
  'desk-check-pass',
  'rework',
  'rejected',
  'ready'
])

const isLifecycleLabel = (label: string): boolean => LIFECYCLE_LABELS.has(label)

type Props = {
  story: Story
  isOwner: boolean
  apiBase: string | undefined
  authToken: string | undefined
  ownerNonce?: string | undefined
  visible: boolean
  focusTimeline?: boolean
  onBack: () => void
  onStoryMutated: (story: Story) => void
  onUnmountReady?: () => void
}

const TOAST_MS = 4000

const DrillIn = ({
  story,
  isOwner,
  apiBase,
  authToken,
  ownerNonce,
  visible,
  focusTimeline,
  onBack,
  onStoryMutated,
  onUnmountReady
}: Props): React.JSX.Element => {
  const [mode, setMode] = useState<ActionMode>('idle')
  const [toast, setToast] = useState<string | null>(null)
  const articleRef = useRef<HTMLElement | null>(null)

  useEffect(() => {
    articleRef.current?.focus()
  }, [])

  useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), TOAST_MS)
    return () => clearTimeout(id)
  }, [toast])

  const canAct = isOwner && story.state === 'acceptance'

  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.defaultPrevented) return
      if (event.key !== 'Escape') return
      if (mode === 'rejecting') return
      event.preventDefault()
      if (mode === 'bug-question') {
        setMode('idle')
        return
      }
      onBack()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [mode, onBack])

  const type = storyTypeFromLabels(story.labels)

  return (
    <article
      className="drill-in"
      data-visible={visible || undefined}
      aria-hidden={!visible || undefined}
      ref={articleRef}
      data-test="board-drill-in"
      aria-label={`Story ${story.id}: ${story.title}`}
      onTransitionEnd={(event) => {
        if (!visible && event.propertyName === 'opacity' && onUnmountReady) {
          onUnmountReady()
        }
      }}
    >
      <header className="drill-in-header">
        <button
          type="button"
          className="drill-in-back"
          onClick={onBack}
          aria-label="Back to board list"
        >
          <BackArrow />
          <small className="drill-in-crumb">Board · #{story.id}</small>
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

      <Comments storyId={story.id} apiBase={apiBase} authToken={authToken} onError={setToast} />

      <Timeline story={story} apiBase={apiBase} focusTimeline={focusTimeline} />

      <DrillInActions
        story={story}
        canAct={canAct}
        apiBase={apiBase}
        authToken={authToken}
        ownerNonce={ownerNonce}
        mode={mode}
        setMode={setMode}
        onStoryMutated={onStoryMutated}
        onBack={onBack}
        onError={setToast}
      />

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
