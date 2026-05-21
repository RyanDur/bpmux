import MiniBoard from './MiniBoard'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useEpicCollapse } from './useEpicCollapse'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { deriveEpicState, type BoardStory, type StoryState } from './types'

interface Props {
  epics: BoardStory[]
  stories: BoardStory[]
  onOpen: (n: number) => void
}

interface StateBreakdown {
  total: number
  accepted: number
  retired: number
  delivered: number
  inProgress: number
  deskCheck: number
  ready: number
  icebox: number
}

const breakdownFor = (epic: BoardStory, all: BoardStory[]): StateBreakdown => {
  const children = all.filter((s) => s.parent_epic === epic.number)
  const count = (state: StoryState): number => children.filter((s) => s.state === state).length
  return {
    total: children.length,
    accepted: count('accepted'),
    retired: count('retired'),
    delivered: count('delivered') + count('ready-for-accept'),
    inProgress: count('in-progress') + count('needs-fixes'),
    deskCheck: count('desk-check'),
    ready: count('ready') + count('rejected'),
    icebox: count('icebox')
  }
}

const progressDenominatorFor = (b: StateBreakdown): number =>
  b.accepted + b.delivered + b.inProgress + b.deskCheck + b.ready + b.icebox

const extractDescription = (body: string): string => {
  if (!body.trim()) return ''
  const userJobMatch = body.match(/##\s+User job\s*\n+([\s\S]+?)(?:\n##\s|$)/i)
  if (userJobMatch && userJobMatch[1]) {
    const para = userJobMatch[1].trim().split(/\n{2,}/)[0]
    if (para) return para.trim()
  }
  const firstPara = body.trim().split(/\n{2,}/)[0]
  return firstPara ? firstPara.trim() : ''
}

const EPIC_TITLE_PREFIX_RE = /^\s*\[\s*epic\s*\]\s*/i

const stripEpicPrefix = (title: string): string => title.replace(EPIC_TITLE_PREFIX_RE, '')

const activeChildrenCountFor = (b: StateBreakdown): number =>
  b.delivered + b.inProgress + b.deskCheck + b.ready + b.icebox

const breakdownAriaLabel = (b: StateBreakdown): string => {
  const denom = progressDenominatorFor(b)
  return `Progress: ${b.accepted} of ${denom} children needing completion accepted; ${b.delivered} delivered, ${b.inProgress} in progress, ${b.deskCheck} in desk-check, ${b.ready} ready, ${b.icebox} in icebox, ${b.retired} retired (scope-removed; excluded from progress denominator)`
}

const isOpenByDefault = (derived: StoryState): boolean =>
  derived !== 'accepted' && derived !== 'icebox'

const Epic = ({ epics, stories, onOpen }: Props): React.JSX.Element | null => {
  const collapse = useEpicCollapse(RENDERER_IDENTITY)
  if (epics.length === 0) return null
  return (
    <section className="board-epics" data-test="board-epics">
      <header className="board-epics-header">
        <b className="board-group-label">Epics</b>
        <small className="board-group-count">{epics.length}</small>
      </header>
      <ul className="board-epics-list">
        {epics.map((epic) => {
          const breakdown = breakdownFor(epic, stories)
          const children = stories.filter((s) => s.parent_epic === epic.number)
          const derived = deriveEpicState(epic.state, children)
          const description = extractDescription(epic.body)
          const defaultOpen = isOpenByDefault(derived)
          const open = collapse.isOpen(epic.number, defaultOpen)
          const denom = progressDenominatorFor(breakdown)
          const activeCount = activeChildrenCountFor(breakdown)
          const allDone = denom > 0 && breakdown.accepted === denom
          const displayTitle = stripEpicPrefix(epic.title)
          return (
            <li key={epic.number} className="board-epic-item">
              <details
                className="board-epic-card"
                data-test={`board-epic-${epic.number}`}
                open={open}
                onToggle={(event) => {
                  collapse.setOpen(epic.number, event.currentTarget.open)
                }}
              >
                <summary className="board-epic-summary">
                  <button
                    type="button"
                    className="board-epic-open-button"
                    onClick={(event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      onOpen(epic.number)
                    }}
                    title={displayTitle}
                    aria-label={`open epic #${epic.number} ${displayTitle}`}
                  >
                    <StoryTypeGlyph type="epic" />
                    <small className="board-story-number">#{epic.number}</small>
                    <span
                      className="board-epic-state"
                      data-state={derived}
                      data-test={`board-epic-${epic.number}-state`}
                    >
                      {derived}
                    </span>
                    <b className="board-epic-title">{displayTitle}</b>
                  </button>
                  <progress
                    className="board-epic-progress board-epic-progress--summary"
                    data-test={`board-epic-${epic.number}-progress`}
                    value={breakdown.accepted}
                    max={denom === 0 ? 1 : denom}
                    aria-label={breakdownAriaLabel(breakdown)}
                  />
                  <small
                    className="board-epic-summary-progress"
                    data-test={`board-epic-${epic.number}-summary-progress`}
                    aria-label={breakdownAriaLabel(breakdown)}
                  >
                    {denom === 0 && breakdown.retired === 0 ? (
                      <span
                        className="board-epic-count board-epic-count--empty"
                        data-test={`board-epic-${epic.number}-count-empty`}
                      >
                        empty
                      </span>
                    ) : (
                      <>
                        <span
                          className="board-epic-count board-epic-count--accepted"
                          data-test={`board-epic-${epic.number}-count-accepted`}
                        >
                          {breakdown.accepted}✓
                        </span>
                        {breakdown.retired > 0 && (
                          <span
                            className="board-epic-count board-epic-count--retired"
                            data-test={`board-epic-${epic.number}-count-retired`}
                          >
                            {' / '}
                            {breakdown.retired}⊘
                          </span>
                        )}
                        {activeCount > 0 && (
                          <span
                            className="board-epic-count board-epic-count--active"
                            data-test={`board-epic-${epic.number}-count-active`}
                          >
                            {' / '}
                            {activeCount}●
                          </span>
                        )}
                        {allDone && (
                          <span
                            className="board-epic-count board-epic-count--done"
                            data-test={`board-epic-${epic.number}-count-done`}
                          >
                            {' ✓'}
                          </span>
                        )}
                      </>
                    )}
                  </small>
                </summary>
                <div className="board-epic-body">
                  {description !== '' && (
                    <p
                      className="board-epic-description"
                      data-test={`board-epic-${epic.number}-description`}
                    >
                      {description}
                    </p>
                  )}
                  <MiniBoard epicNumber={epic.number} stories={children} />
                </div>
              </details>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export default Epic
