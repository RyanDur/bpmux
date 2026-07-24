import { useCallback, useMemo, useState } from 'react'
import { useCollapsibleSectionsStorage } from './useCollapsibleSectionsStorage'
import { RENDERER_IDENTITY, useBoardColumns } from '../state'
import { useDispatch } from '../state/useDispatch'
import SyncStatusGlyph from '../sync/SyncStatusGlyph'
import DrillIn from './drill-in/DrillIn'
import BoardTabs from './tabs/BoardTabs'
import BoardRoadmapSwitcher from './tabs/BoardRoadmapSwitcher'
import Roadmap from './roadmap/Roadmap'
import { useBoardData } from './useBoardData'
import { useBoardFetch } from './useBoardFetch'
import { useBoardNavigation } from './useBoardNavigation'
import { useBoardReorder } from './drag/useBoardReorder'
import { findCurrentIteration } from './iteration/currentIteration'
import { deriveCloseApproachNudge } from './nudge/deriveCloseApproachNudge'
import { useIterations } from './iteration/useIterations'
import { useEpics } from './roadmap/useEpics'
import { iterationNameFromTitle } from './iteration/labelSchema'
import { GROUP_ORDER, type Story, type ColumnState } from './types'
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

type BoardProps = {
  hidden?: boolean
  width?: number
}

const Board = ({ hidden, width }: BoardProps): React.JSX.Element => {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const { data, setData, loading, error, refetch } = useBoardFetch({
    apiBase: bridge?.apiBase
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

  const iterations = useIterations()
  const currentIteration = findCurrentIteration(iterations)
  const currentIterationStories = currentIteration?.stories ?? null
  const [pendingReady, setPendingReady] = useState<{
    order: number[]
    basedOn: number[] | null
  } | null>(null)
  const setPendingReadyOrder = (order: number[]): void =>
    setPendingReady({ order, basedOn: currentIterationStories })
  const readyOrder =
    pendingReady !== null && pendingReady.basedOn === currentIterationStories
      ? pendingReady.order
      : (currentIterationStories ?? [])
  const wireEpics = useEpics()
  const columns = useBoardColumns()
  const {
    stories,
    epics,
    epicTitleById,
    iterationContainerStories,
    grouped,
    groupOrder,
    navOrder
  } = useBoardData(data, iterations, readyOrder, wireEpics, columns)

  const currentIterationName =
    currentIteration !== null ? iterationNameFromTitle(currentIteration.title) : null
  const closeApproachNudge = useMemo(
    () => deriveCloseApproachNudge(iterations, stories),
    [iterations, stories]
  )

  const {
    asideRef,
    setSelectedId,
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
    (updated: Story) => {
      setData((prev) => {
        if (!prev) return prev
        return prev.map((s) => (s.id === updated.id ? updated : s))
      })
    },
    [setData]
  )

  const dispatch = useDispatch()
  const handleReorder = useBoardReorder({
    data,
    setData,
    setReorderAnnounce,
    dispatch,
    currentIterationId: currentIteration?.id ?? null,
    currentIterationStories: currentIterationStories ?? [],
    setPendingReadyOrder
  })

  const isOwner = RENDERER_IDENTITY === 'owner'

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
      <output
        className="sr-only"
        aria-live="polite"
        aria-label="Reorder announcements"
        data-test="board-reorder-announce"
      >
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
            <SyncStatusGlyph />
          </>
        }
        board={
          renderedDrill !== null ? (
            <DrillIn
              story={renderedDrill}
              isOwner={isOwner}
              apiBase={bridge?.apiBase}
              authToken={bridge?.authToken}
              ownerNonce={bridge?.ownerNonce}
              visible={drilledStory !== null}
              focusTimeline={focusTimelineFor === renderedDrill.id}
              onBack={handleBack}
              onStoryMutated={handleStoryMutated}
              onUnmountReady={handleDrillUnmountReady}
            />
          ) : (
            <section className="board-body" data-test="board-list">
              <BoardTabs
                stories={stories}
                iterations={iterations}
                groupOrder={groupOrder}
                epics={epics}
                epicTitleById={epicTitleById}
                iterationContainerStories={iterationContainerStories}
                grouped={grouped}
                loading={loading && !data}
                effectiveSelected={effectiveSelected}
                collapsed={collapsed}
                onToggleCollapse={toggleCollapsed}
                onOpen={handleOpen}
                onOpenTimeline={handleOpenTimeline}
                onSelect={setSelectedId}
                onReorder={handleReorder}
                onReorderAnnounce={setReorderAnnounce}
              />
            </section>
          )
        }
        roadmap={<Roadmap />}
      />
    </aside>
  )
}

export default Board
