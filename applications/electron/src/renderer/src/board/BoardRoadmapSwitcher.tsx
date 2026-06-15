import { useId } from 'react'
import { useTabStorage } from '../lib/useTabStorage'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import './BoardRoadmapSwitcher.css'

export type BoardRoadmapView = 'board' | 'roadmap'

const STORAGE_PREFIX = 'bpmux-board-roadmap-view'
const VALID_VIEWS: ReadonlyArray<BoardRoadmapView> = ['board', 'roadmap'] as const

interface BoardRoadmapSwitcherProps {
  boardLabel: React.ReactNode
  mock?: React.ReactNode
  board: React.ReactNode
  roadmap: React.ReactNode
}

const BoardRoadmapSwitcher = ({
  boardLabel,
  mock,
  board,
  roadmap
}: BoardRoadmapSwitcherProps): React.JSX.Element => {
  const { activeId, setActiveId } = useTabStorage<BoardRoadmapView>(
    STORAGE_PREFIX,
    RENDERER_IDENTITY,
    VALID_VIEWS,
    'board'
  )
  const groupName = useId()
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
          </label>
          {mock}
        </fieldset>
      </header>
      <article
        className="tabs-panel board-roadmap-panel"
        data-test="board-roadmap-panel-board"
        hidden={activeId !== 'board'}
      >
        {board}
      </article>
      <article
        className="tabs-panel board-roadmap-panel"
        data-test="board-roadmap-panel-roadmap"
        hidden={activeId !== 'roadmap'}
      >
        {roadmap}
      </article>
    </section>
  )
}

export default BoardRoadmapSwitcher
