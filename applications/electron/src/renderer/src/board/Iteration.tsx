import { useState } from 'react'
import StoryTypeGlyph from './StoryTypeGlyph'
import { useIterationCollapse } from './useIterationCollapse'
import { computeDropMutation } from './iterationPosition'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { storyTypeFromLabels, type BoardStory } from './types'

interface Props {
  stories: BoardStory[]
  onOpen: (n: number) => void
}

const UNSCHEDULED_KEY = 'unscheduled'
const UNSCHEDULED_LABEL_PREFIX = 'iteration-Unscheduled.'

const isIterationStory = (story: BoardStory): boolean =>
  storyTypeFromLabels(story.labels) === 'iteration'

const themeNameFromTitle = (title: string): string => {
  const match = title.match(/^Iteration (.+)$/)
  const raw = match ? match[1].trim() : title.trim()
  return raw.replace(/\s+/g, '-')
}

interface IterationLabelParts {
  name: string
  position: number
}

const parseIterationLabel = (label: string): IterationLabelParts | null => {
  const match = label.match(/^iteration-(.+)\.(\d+)$/)
  if (!match) return null
  return { name: match[1], position: parseInt(match[2], 10) }
}

const positionInIteration = (story: BoardStory, themeName: string): number | null => {
  for (const label of story.labels) {
    const parsed = parseIterationLabel(label)
    if (parsed?.name === themeName) return parsed.position
  }
  return null
}

const positionInUnscheduled = (story: BoardStory): number | null => {
  for (const label of story.labels) {
    if (label.startsWith(UNSCHEDULED_LABEL_PREFIX)) {
      const tail = label.slice(UNSCHEDULED_LABEL_PREFIX.length)
      const n = parseInt(tail, 10)
      if (!Number.isNaN(n)) return n
    }
  }
  return null
}

const isLooseStory = (story: BoardStory): boolean => {
  if (isIterationStory(story)) return false
  if (story.state !== 'icebox') return false
  const hasNamedIterationLabel = story.labels.some((l) => {
    const parsed = parseIterationLabel(l)
    return parsed !== null && parsed.name !== 'Unscheduled'
  })
  return !hasNamedIterationLabel
}

interface DragPayload {
  storyNumber: number
  themeName: string
  oldPosition: number
}

const DRAG_MIMETYPE = 'application/x-bpmux-iteration-member'

const postLabelMutation = (
  storyNumber: number,
  addLabels: string[],
  removeLabels: string[]
): void => {
  if (typeof window === 'undefined') return
  const bridge = window.bpmux
  if (!bridge?.apiBase) return
  void fetch(`${bridge.apiBase}/api/stories/${storyNumber}/labels`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${bridge.authToken ?? ''}`
    },
    body: JSON.stringify({ add: addLabels, remove: removeLabels })
  })
}

const Iteration = ({ stories, onOpen }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const [draggedNumber, setDraggedNumber] = useState<number | null>(null)
  const iterations = stories.filter(isIterationStory)
  const loose = stories.filter(isLooseStory)
  if (iterations.length === 0 && loose.length === 0) return null

  const sortedLoose = [...loose].sort((a, b) => {
    const ap = positionInUnscheduled(a)
    const bp = positionInUnscheduled(b)
    if (ap !== null && bp !== null) return ap - bp
    if (ap !== null) return -1
    if (bp !== null) return 1
    return a.number - b.number
  })

  return (
    <section className="board-iterations" data-test="board-iterations">
      <ul className="board-iterations-list">
        {iterations.map((iteration) => {
          const key = String(iteration.number)
          const defaultOpen = iteration.gh_state !== 'closed'
          const open = collapse.isOpen(key, defaultOpen)
          const themeName = themeNameFromTitle(iteration.title)
          const members = stories
            .map((s) => {
              if (isIterationStory(s)) return null
              const position = positionInIteration(s, themeName)
              if (position === null) return null
              return { story: s, position }
            })
            .filter((m): m is { story: BoardStory; position: number } => m !== null)
            .sort((a, b) => a.position - b.position)
          return (
            <li key={iteration.number} className="board-iteration-item">
              <details
                className="board-iteration-card"
                data-test={`board-iteration-${iteration.number}`}
                data-iteration-kind="named"
                data-iteration-name={themeName}
                data-closed={iteration.gh_state === 'closed' ? 'true' : undefined}
                open={open}
                onToggle={(event) => {
                  collapse.setOpen(key, event.currentTarget.open)
                }}
              >
                <summary className="board-iteration-summary">
                  <button
                    type="button"
                    className="board-iteration-open-button"
                    onClick={(event) => {
                      event.preventDefault()
                      event.stopPropagation()
                      onOpen(iteration.number)
                    }}
                    title={iteration.title}
                    aria-label={`open iteration #${iteration.number} ${iteration.title}`}
                  >
                    <StoryTypeGlyph type="iteration" />
                    <small className="board-story-number">#{iteration.number}</small>
                    <b className="board-iteration-title">{iteration.title}</b>
                    <small className="board-iteration-count">{members.length}</small>
                  </button>
                </summary>
                <ul className="board-iteration-body">
                  {members.map(({ story, position }, memberIdx) => (
                    <li
                      key={story.number}
                      className="board-iteration-member"
                      data-test={`board-story-${story.number}`}
                      data-iteration-member={story.number}
                      draggable={iteration.gh_state !== 'closed'}
                      data-dragging={draggedNumber === story.number ? 'true' : undefined}
                      onDragStart={(event) => {
                        const payload: DragPayload = {
                          storyNumber: story.number,
                          themeName,
                          oldPosition: position
                        }
                        event.dataTransfer.setData(DRAG_MIMETYPE, JSON.stringify(payload))
                        event.dataTransfer.effectAllowed = 'move'
                        setDraggedNumber(story.number)
                      }}
                      onDragEnd={() => setDraggedNumber(null)}
                      onDragOver={(event) => {
                        if (event.dataTransfer.types.includes(DRAG_MIMETYPE)) {
                          event.preventDefault()
                          event.dataTransfer.dropEffect = 'move'
                        }
                      }}
                      onDrop={(event) => {
                        const raw = event.dataTransfer.getData(DRAG_MIMETYPE)
                        if (!raw) return
                        event.preventDefault()
                        const payload = JSON.parse(raw) as DragPayload
                        if (payload.storyNumber === story.number) return
                        if (iteration.gh_state === 'closed') return
                        const mutation = computeDropMutation({
                          draggedNumber: payload.storyNumber,
                          draggedThemeName: payload.themeName,
                          draggedOldPosition: payload.oldPosition,
                          targetThemeName: themeName,
                          targetMemberPositions: members.map((m) => m.position),
                          targetMemberIndex: memberIdx,
                          targetMembersIncludeDragged: payload.themeName === themeName
                        })
                        if (!mutation) return
                        postLabelMutation(payload.storyNumber, mutation.add, mutation.remove)
                      }}
                    >
                      <button
                        type="button"
                        className="board-iteration-member-button"
                        onClick={() => onOpen(story.number)}
                      >
                        <small className="board-story-number">#{story.number}</small>
                        <span className="board-story-title">{story.title}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </details>
            </li>
          )
        })}
        {sortedLoose.length > 0 && (
          <li key="unscheduled" className="board-iteration-item">
            <details
              className="board-iteration-card board-iteration-card--unscheduled"
              data-test="board-iteration-unscheduled"
              data-iteration-kind="unscheduled"
              open={collapse.isOpen(UNSCHEDULED_KEY, true)}
              onToggle={(event) => {
                collapse.setOpen(UNSCHEDULED_KEY, event.currentTarget.open)
              }}
            >
              <summary className="board-iteration-summary">
                <b className="board-iteration-title">Unscheduled</b>
                <small className="board-iteration-count">{sortedLoose.length}</small>
              </summary>
              <ul className="board-iteration-body">
                {sortedLoose.map((story) => (
                  <li
                    key={story.number}
                    className="board-iteration-member"
                    data-test={`board-story-${story.number}`}
                    data-iteration-member={story.number}
                  >
                    <button
                      type="button"
                      className="board-iteration-member-button"
                      onClick={() => onOpen(story.number)}
                    >
                      <small className="board-story-number">#{story.number}</small>
                      <span className="board-story-title">{story.title}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </details>
          </li>
        )}
      </ul>
    </section>
  )
}

export default Iteration
