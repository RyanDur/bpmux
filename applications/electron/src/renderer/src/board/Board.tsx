import { useCallback, useMemo, useState } from 'react'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import SyncStatusGlyph from '../sync/SyncStatusGlyph'
import DrillIn from './DrillIn'
import BoardTabs from './BoardTabs'
import BoardRoadmapSwitcher from './BoardRoadmapSwitcher'
import Roadmap from './Roadmap'
import { useBoardData } from './useBoardData'
import { useBoardFetch } from './useBoardFetch'
import { useBoardNavigation } from './useBoardNavigation'
import { useBoardReorder } from './useBoardReorder'
import { usePendingReadyOrder } from './usePendingReadyOrder'
import { findCurrentIteration } from './currentIteration'
import { deriveCloseApproachNudge } from './deriveCloseApproachNudge'
import { themeNameFromTitle } from './labelSchema'
import { GROUP_ORDER, type BoardResponse, type BoardStory, type ColumnState } from './types'
import './Board.css'

type CollapseState = Record<ColumnState, boolean>

const BOARD_COLLAPSE_STORAGE_PREFIX = 'bpmux-board-collapse'

const initialCollapseState: CollapseState = (() => {
  const result: Record<string, boolean> = {}
  for (const group of GROUP_ORDER) {
    result[group.state] = group.collapsed
  }
  return result as CollapseState
})()

interface BoardProps {
  boardSnapshot?: BoardResponse | null
  boardInvalidateTick: number
  hidden?: boolean
  width?: number
}

const Board = ({
  boardSnapshot,
  boardInvalidateTick,
  hidden,
  width
}: BoardProps): React.JSX.Element => {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const { data, setData, loading, error, setError, usingMock, refetch } = useBoardFetch({
    boardSnapshot,
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })
  const [reorderAnnounce, setReorderAnnounce] = useState<string>('')
  const collapseStorage = useCollapsibleSectionsStorage<ColumnState>(
    BOARD_COLLAPSE_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    initialCollapseState
  )
  const collapsed = collapseStorage.state
  const toggleCollapsed = useCallback(
    (state: ColumnState): void => {
      collapseStorage.setState({ ...collapseStorage.state, [state]: !collapseStorage.state[state] })
    },
    [collapseStorage]
  )

  const { setPendingReadyOrder, clearPendingReadyOrder, effectiveReadyPosition } =
    usePendingReadyOrder(data)
  const { stories, epics, epicTitleByNumber, iterationContainerStories, grouped, navOrder } =
    useBoardData(data, effectiveReadyPosition)

  const currentIteration = findCurrentIteration(stories)
  const currentIterationName =
    currentIteration !== null ? themeNameFromTitle(currentIteration.title) : null
  const closeApproachNudge = useMemo(() => deriveCloseApproachNudge(stories), [stories])

  const {
    asideRef,
    setSelectedNumber,
    drilledStory,
    focusTimelineFor,
    effectiveSelected,
    renderedDrill,
    handleOpen,
    handleOpenTimeline,
    handleBack,
    handleDrillUnmountReady,
    handleKeyDown
  } = useBoardNavigation(stories, navOrder)

  const handleStoryMutated = useCallback(
    (updated: BoardStory) => {
      setData((prev) => {
        if (!prev) return prev
        return {
          ...prev,
          stories: prev.stories.map((s) => (s.number === updated.number ? updated : s))
        }
      })
    },
    [setData]
  )

  const handleReorder = useBoardReorder({
    data,
    setData,
    setReorderAnnounce,
    setError,
    setPendingReadyOrder,
    clearPendingReadyOrder,
    usingMock,
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })

  const isOwner = RENDERER_IDENTITY === 'ryan'

  return (
    <aside
      id="board-region"
      className={`board${hidden ? ' board--hidden' : ''}`}
      data-test="board-region"
      ref={asideRef}
      onKeyDown={handleKeyDown}
      aria-label="Board"
      aria-hidden={hidden ? true : undefined}
      style={width !== undefined && !hidden ? { flex: `0 0 ${width}px` } : undefined}
      inert={hidden ? true : undefined}
    >
      <output className="sr-only" aria-live="polite" data-test="board-reorder-announce">
        {reorderAnnounce}
      </output>

      {error !== null && (
        <section className="board-error" role="alert" data-test="board-error">
          <p className="board-error-text">Couldn&apos;t refresh board: {error}</p>
          <button type="button" className="board-error-retry" onClick={refetch}>
            Retry
          </button>
        </section>
      )}

      <BoardRoadmapSwitcher
        nudge={closeApproachNudge}
        currentIterationName={currentIterationName}
        boardLabel={
          <>
            {currentIterationName !== null && (
              <strong className="board-iteration-name" data-test="board-header-iteration">
                {currentIterationName}
              </strong>
            )}
            <small className="board-title">Board</small>
            <SyncStatusGlyph apiBase={bridge?.apiBase} authToken={bridge?.authToken} />
          </>
        }
        mock={
          usingMock ? (
            <small
              className="board-mock-flag"
              aria-label="showing mock data"
              data-test="board-mock-flag"
            >
              mock
            </small>
          ) : undefined
        }
        board={
          renderedDrill !== null ? (
            <DrillIn
              story={renderedDrill}
              isOwner={isOwner}
              apiBase={bridge?.apiBase}
              authToken={bridge?.authToken}
              usingMock={usingMock}
              boardInvalidateTick={boardInvalidateTick}
              visible={drilledStory !== null}
              focusTimeline={focusTimelineFor === renderedDrill.number}
              onBack={handleBack}
              onStoryMutated={handleStoryMutated}
              onUnmountReady={handleDrillUnmountReady}
            />
          ) : (
            <section className="board-body" data-test="board-list">
              <BoardTabs
                stories={stories}
                epics={epics}
                epicTitleByNumber={epicTitleByNumber}
                iterationContainerStories={iterationContainerStories}
                grouped={grouped}
                loading={loading && !data}
                effectiveSelected={effectiveSelected}
                collapsed={collapsed}
                onToggleCollapse={toggleCollapsed}
                onOpen={handleOpen}
                onOpenTimeline={handleOpenTimeline}
                onSelect={setSelectedNumber}
                onReorder={handleReorder}
                onReorderAnnounce={setReorderAnnounce}
              />
            </section>
          )
        }
        roadmap={<Roadmap stories={stories} />}
      />
    </aside>
  )
}

export default Board
