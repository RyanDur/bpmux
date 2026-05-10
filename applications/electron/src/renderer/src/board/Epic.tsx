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
    delivered: count('delivered') + count('ready-for-accept'),
    inProgress: count('in-progress') + count('needs-fixes'),
    deskCheck: count('desk-check'),
    ready: count('ready') + count('rejected'),
    icebox: count('icebox')
  }
}

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

const breakdownAriaLabel = (b: StateBreakdown): string =>
  `Progress: ${b.accepted} of ${b.total} children accepted; ${b.delivered} delivered, ${b.inProgress} in progress, ${b.deskCheck} in desk-check, ${b.ready} ready, ${b.icebox} in icebox`

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
          const allDone = breakdown.total > 0 && breakdown.accepted === breakdown.total
          const summaryProgress = `${breakdown.accepted}/${breakdown.total === 0 ? 0 : breakdown.total}${allDone ? ' ✓' : ''}`
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
                    title={epic.title}
                    aria-label={`open epic #${epic.number} ${epic.title}`}
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
                    <b className="board-epic-title">{epic.title}</b>
                  </button>
                  <progress
                    className="board-epic-progress board-epic-progress--summary"
                    data-test={`board-epic-${epic.number}-progress`}
                    value={breakdown.accepted}
                    max={breakdown.total === 0 ? 1 : breakdown.total}
                    aria-label={breakdownAriaLabel(breakdown)}
                  />
                  <small
                    className="board-epic-summary-progress"
                    data-test={`board-epic-${epic.number}-summary-progress`}
                  >
                    {summaryProgress}
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
