import StoryTypeGlyph from './StoryTypeGlyph'
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
  const count = (state: StoryState): number =>
    children.filter((s) => s.state === state).length
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

const breakdownText = (b: StateBreakdown): string =>
  [
    `${b.accepted}/${b.total} accepted`,
    `${b.delivered}/${b.total} delivered`,
    `${b.inProgress}/${b.total} in-progress`,
    `${b.deskCheck}/${b.total} desk-check`,
    `${b.ready}/${b.total} ready`,
    `${b.icebox}/${b.total} icebox`
  ].join(' · ')

const Epic = ({ epics, stories, onOpen }: Props): React.JSX.Element | null => {
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
          return (
            <li key={epic.number} className="board-epic-item">
              <article
                className="board-epic-card"
                data-test={`board-epic-${epic.number}`}
              >
                <header className="board-epic-header">
                  <button
                    type="button"
                    className="board-epic-button"
                    onClick={() => onOpen(epic.number)}
                    title={epic.title}
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
                </header>
                {description !== '' && (
                  <p
                    className="board-epic-description"
                    data-test={`board-epic-${epic.number}-description`}
                  >
                    {description}
                  </p>
                )}
                <progress
                  className="board-epic-progress"
                  data-test={`board-epic-${epic.number}-progress`}
                  value={breakdown.accepted}
                  max={breakdown.total === 0 ? 1 : breakdown.total}
                  aria-label={breakdownAriaLabel(breakdown)}
                />
                <small
                  className="board-epic-breakdown"
                  data-test={`board-epic-${epic.number}-breakdown`}
                >
                  {breakdownText(breakdown)}
                </small>
              </article>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export default Epic
