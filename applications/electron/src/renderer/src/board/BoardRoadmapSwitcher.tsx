import { useId, useState } from 'react'
import { useTabStorage } from '../lib/useTabStorage'
import { RENDERER_IDENTITY } from '../transport/session'
import CloseApproachNudge from './CloseApproachNudge'
import type { CloseApproachNudge as CloseApproachNudgeData } from './deriveCloseApproachNudge'
import './BoardRoadmapSwitcher.css'

export type BoardRoadmapView = 'board' | 'roadmap'

const STORAGE_PREFIX = 'bpmux-board-roadmap-view'
const VALID_VIEWS: ReadonlyArray<BoardRoadmapView> = ['board', 'roadmap'] as const

interface BoardRoadmapSwitcherProps {
  boardLabel: React.ReactNode
  mock?: React.ReactNode
  board: React.ReactNode
  roadmap: React.ReactNode
  nudge?: CloseApproachNudgeData | null
  currentIterationName?: string | null
}

const BoardRoadmapSwitcher = ({
  boardLabel,
  mock,
  board,
  roadmap,
  nudge,
  currentIterationName
}: BoardRoadmapSwitcherProps): React.JSX.Element => {
  const { activeId, setActiveId } = useTabStorage<BoardRoadmapView>(
    STORAGE_PREFIX,
    RENDERER_IDENTITY,
    VALID_VIEWS,
    'board'
  )
  const groupName = useId()
  const [dismissedNextNumber, setDismissedNextNumber] = useState<number | null>(null)
  return (
    <section className="board-roadmap-switcher tabs">
      <header className="board-roadmap-tabrow">
        <fieldset className="board-roadmap-fieldset">
          <legend className="tabs-legend">Board or Roadmap</legend>
          <label
            className="board-roadmap-tab board-roadmap-tab--board"
            data-test="board-roadmap-tab-board"
            data-active={activeId === 'board' ? 'true' : undefined}
          >
            <input
              type="radio"
              name={groupName}
              checked={activeId === 'board'}
              onChange={() => setActiveId('board')}
              className="tabs-radio"
            />
            {boardLabel}
          </label>
          <label
            className="board-roadmap-tab"
            data-test="board-roadmap-tab-roadmap"
            data-active={activeId === 'roadmap' ? 'true' : undefined}
          >
            <input
              type="radio"
              name={groupName}
              checked={activeId === 'roadmap'}
              onChange={() => setActiveId('roadmap')}
              className="tabs-radio"
            />
            🗺 Roadmap
            {nudge != null && (
              <i className="board-roadmap-attn" data-test="board-roadmap-attn" aria-hidden="true">
                ◐
              </i>
            )}
          </label>
          {mock}
        </fieldset>
      </header>
      {nudge != null && currentIterationName != null && (
        <CloseApproachNudge
          currentIterationName={currentIterationName}
          nextIterationName={nudge.nextIterationName}
          activeGapLabel={nudge.activeGapLabel}
          dismissed={nudge.nextIterationNumber === dismissedNextNumber}
          hidden={activeId !== 'board'}
          onViewRoadmap={() => setActiveId('roadmap')}
          onDismiss={() => setDismissedNextNumber(nudge.nextIterationNumber)}
        />
      )}
      <article
        className="tabs-panel board-roadmap-panel"
        data-test="board-roadmap-panel-board"
        aria-label="Board"
        hidden={activeId !== 'board'}
      >
        {board}
      </article>
      {activeId === 'roadmap' && (
        <article
          className="tabs-panel board-roadmap-panel"
          data-test="board-roadmap-panel-roadmap"
          aria-label="Roadmap"
        >
          {roadmap}
        </article>
      )}
    </section>
  )
}

export default BoardRoadmapSwitcher
