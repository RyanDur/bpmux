import { useCallback, useMemo, useRef, useState } from 'react'
import { useAnimatedUnmount } from '../hooks/useAnimatedUnmount'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import type { ChatMessage, ConnectionStatus } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import SyncStatusGlyph from '../sync/SyncStatusGlyph'
import DrillIn from './DrillIn'
import Epic from './Epic'
import Iteration from './Iteration'
import BoardGroup from './BoardGroup'
import { useBoardData } from './useBoardData'
import { useBoardFetch } from './useBoardFetch'
import { useBoardReorder } from './useBoardReorder'
import { GROUP_ORDER, type BoardStory, type ColumnState } from './types'
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
  // Chat message stream — forwarded to DrillIn so it can subscribe to
  // BOARD_INVALIDATE_MARKER broadcasts for live comment refresh.
  // Board itself doesn't observe messages; Board polls /api/board on
  // a 30s interval for story list changes, which is orthogonal.
  messages: ChatMessage[]
  // Collapsed state for Ctrl+P toggle. When true, the aside stays
  // mounted (state preserved) but animates to 0 width via CSS. When
  // false, the aside is visible at its normal width. See Board.css
  // for the transition declaration (#351 AC3).
  hidden?: boolean
  width?: number
  wsStatus?: ConnectionStatus
}

const Board = ({ messages, hidden, width, wsStatus }: BoardProps): React.JSX.Element => {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const { data, setData, loading, error, setError, usingMock, refetch } = useBoardFetch({
    messages,
    wsStatus,
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })
  const [selectedNumber, setSelectedNumber] = useState<number | null>(null)
  const [drilledNumber, setDrilledNumber] = useState<number | null>(null)
  // #368: when a team-accept badge is clicked, open the drill-in AND
  // auto-expand + scroll the timeline. Tracked as a story number (so
  // subsequent re-clicks on the same story re-fire even when that
  // story's drill-in is already open). Cleared on drill close.
  const [focusTimelineFor, setFocusTimelineFor] = useState<number | null>(null)
  // Reorder announcement (#357 AC5). Updated on each keyboard-driven
  // reorder so screen readers narrate "moved to position N".
  // aria-live="polite" queues the announcement without interrupting
  // what the user is currently reading.
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

  const asideRef = useRef<HTMLElement | null>(null)

  const { stories, epics, epicTitleByNumber, iterationContainerStories, grouped, navOrder } =
    useBoardData(data)

  // Derive the effective selection at render time instead of syncing
  // state in an effect — if the stored selection has scrolled off the
  // list we show the first row as highlighted. Keyboard-nav handlers
  // still commit the user's explicit choice via setSelectedNumber.
  const effectiveSelected =
    selectedNumber !== null && navOrder.includes(selectedNumber)
      ? selectedNumber
      : (navOrder[0] ?? null)

  const drilledStory = useMemo(
    () =>
      drilledNumber === null ? null : (stories.find((s) => s.number === drilledNumber) ?? null),
    [drilledNumber, stories]
  )

  // Close-direction animation (#354 AC1): when drilledStory flips
  // null, keep DrillIn mounted for the exit transition using the
  // last-known story, then unmount. Snapshot the story in a ref so
  // the fade-out has a stable object to render while drilledStory
  // itself is null.
  const [lastDrilledStory, setLastDrilledStory] = useState<typeof drilledStory>(drilledStory)
  if (drilledStory !== null && drilledStory !== lastDrilledStory) {
    setLastDrilledStory(drilledStory)
  }
  const drillAnim = useAnimatedUnmount(drilledStory !== null, 250)
  const renderedDrill = drilledStory ?? lastDrilledStory

  const handleOpen = useCallback((number: number) => {
    setSelectedNumber(number)
    setDrilledNumber(number)
    setFocusTimelineFor(null)
  }, [])

  const handleOpenTimeline = useCallback((number: number) => {
    setSelectedNumber(number)
    setDrilledNumber(number)
    setFocusTimelineFor(number)
  }, [])

  const handleBack = useCallback(() => {
    setDrilledNumber(null)
    setFocusTimelineFor(null)
    // Refocus the board so arrow nav resumes immediately.
    requestAnimationFrame(() => asideRef.current?.focus())
  }, [])

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
    usingMock,
    apiBase: bridge?.apiBase,
    authToken: bridge?.authToken
  })

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      // Drill-in owns its own key handling; don't double-dispatch.
      if (drilledNumber !== null) {
        if (event.key === 'Escape') {
          event.preventDefault()
          handleBack()
        }
        return
      }
      if (navOrder.length === 0) return
      const current = effectiveSelected
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        const idx = current === null ? -1 : navOrder.indexOf(current)
        const next = navOrder[Math.min(idx + 1, navOrder.length - 1)]
        setSelectedNumber(next)
      } else if (event.key === 'ArrowUp') {
        event.preventDefault()
        const idx = current === null ? navOrder.length : navOrder.indexOf(current)
        const next = navOrder[Math.max(idx - 1, 0)]
        setSelectedNumber(next)
      } else if (event.key === 'Enter') {
        const target = event.target as HTMLElement
        if (target.closest('summary')) return
        if (current !== null) {
          event.preventDefault()
          handleOpen(current)
        }
      }
    },
    [drilledNumber, effectiveSelected, handleBack, handleOpen, navOrder]
  )

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
      // inert removes the element from focus + pointer events when
      // animating out; aria-hidden keeps AT from traversing.
      inert={hidden ? true : undefined}
    >
      <header className="board-header">
        <SyncStatusGlyph
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          messages={messages}
        />
        <h2 className="board-title">Board</h2>
        {usingMock ? (
          <small
            className="board-mock-flag"
            aria-label="showing mock data"
            data-test="board-mock-flag"
          >
            mock
          </small>
        ) : null}
      </header>
      {/* Reorder live-region (#357 AC5). Visually hidden; SR users
       * hear the new position after each keyboard reorder. aria-live
       * polite queues without interrupting the current announcement
       * (e.g., the aria-label the focused story button just
       * updated to). */}
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

      {drillAnim.mounted && renderedDrill !== null ? (
        <DrillIn
          story={renderedDrill}
          isOwner={isOwner}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          usingMock={usingMock}
          messages={messages}
          visible={drillAnim.visible}
          focusTimeline={focusTimelineFor === renderedDrill.number}
          onBack={handleBack}
          onStoryMutated={handleStoryMutated}
        />
      ) : (
        <section className="board-body" data-test="board-list">
          <Epic epics={epics} stories={stories} onOpen={handleOpen} />
          {GROUP_ORDER.map((group) => (
            <BoardGroup
              key={group.state}
              group={group}
              stories={
                group.state === 'icebox'
                  ? iterationContainerStories
                  : (grouped.get(group.state) ?? [])
              }
              epicTitleByNumber={epicTitleByNumber}
              loading={loading && !data}
              selectedNumber={effectiveSelected}
              onOpen={handleOpen}
              onOpenTimeline={handleOpenTimeline}
              onSelect={setSelectedNumber}
              onReorder={handleReorder}
              collapsed={collapsed[group.state]}
              onToggleCollapse={() => toggleCollapsed(group.state)}
            >
              {group.state === 'icebox' ? (
                <Iteration
                  stories={iterationContainerStories}
                  onOpen={handleOpen}
                  onReorderAnnounce={setReorderAnnounce}
                />
              ) : undefined}
            </BoardGroup>
          ))}
        </section>
      )}
    </aside>
  )
}

export default Board
