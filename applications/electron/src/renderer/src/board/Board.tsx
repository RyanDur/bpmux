import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useAnimatedUnmount } from '../hooks/useAnimatedUnmount'
import { useCollapsibleSectionsStorage } from '../lib/useCollapsibleSectionsStorage'
import {
  BOARD_INVALIDATE_MARKER,
  type ChatMessage,
  type ConnectionStatus
} from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import { describeHttpError } from '../api/http'
import { getBoard } from '../api/boardClient'
import DrillIn from './DrillIn'
import Epic from './Epic'
import Iteration from './Iteration'
import { isIterationPositionLabel } from './labelSchema'
import BoardGroup from './BoardGroup'
import { useBoardReorder } from './useBoardReorder'
import { useRefetchOnReconnect } from './useRefetchOnReconnect'
import {
  GROUP_ORDER,
  groupStories,
  isActiveEpic,
  storyTypeFromLabels,
  type BoardResponse,
  type BoardStory,
  type ColumnState
} from './types'
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

// Safety-net fallback refetch interval. Primary refresh is the WS
// BOARD_INVALIDATE_MARKER subscription (#353); polling survives as a
// cross-check in case a broadcast is missed (WS reconnect window, a
// backend mutation that skipped the invalidate emit, etc). 5 minutes
// is slow enough to not feel like primary polling but fast enough to
// self-heal within a single work rhythm.
const POLL_INTERVAL_MS = 5 * 60_000

// Mock stories used when `window.bpmux.apiBase` is absent or the real
// endpoint 404s during parallel backend bring-up. The shape must match
// the real BoardResponse contract so the UI rendering path is
// identical in both modes — we log a one-time warning so dev knows
// they're looking at fallback data.
const MOCK_RESPONSE: BoardResponse = {
  fetched_at: new Date().toISOString(),
  stories: [
    {
      number: 324,
      title: 'Side-panel board surface — story list + drill-in',
      state: 'ready',
      labels: ['Feature', 'ready'],
      assignees: ['dev'],
      created_at: '2026-04-18T12:00:00Z',
      body: '## What\nRight-docked board that lists current stories, groups by state, and drills into a single story on click.\n\n## Why\nRyan needs the board visible alongside chat during sessions so story state is continuously present, not a tool-call away.\n\n## Acceptance criteria\n- Groups collapse / expand\n- Arrow nav works\n- Drill-in shows body, accept/reject visible for delivered'
    },
    {
      number: 325,
      title: 'Project panel drill-in: metadata + actions',
      state: 'in-progress',
      labels: ['Feature', 'in-progress'],
      assignees: ['dev'],
      created_at: '2026-04-19T08:30:00Z',
      body: 'Drill-in shows state, labels, assignees, body — plus Accept/Reject for the owner on delivered stories.'
    },
    {
      number: 326,
      title: 'Accept button fires from keyboard enter',
      state: 'delivered',
      labels: ['Bug'],
      assignees: [],
      created_at: '2026-04-20T10:15:00Z',
      body: 'When a delivered story is open + Accept has focus, Enter should fire the accept action. Currently only mouse click works.'
    }
  ]
}

const hasIterationMemberLabel = (labels: string[]): boolean =>
  labels.some((l) => isIterationPositionLabel(l))

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

function Board({ messages, hidden, width, wsStatus }: BoardProps): React.JSX.Element {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [data, setData] = useState<BoardResponse | null>(null)
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)
  const [usingMock, setUsingMock] = useState<boolean>(false)
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

  // TODO(phase-1a-2): drag-reorder (reorder_story) + BOARD_INVALIDATE_MARKER
  // subscription. 30s polling is the phase-1a-1 guarantee.
  const asideRef = useRef<HTMLElement | null>(null)

  const fetchBoard = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      if (!bridge?.apiBase) {
        // No preload / offline dev — render with mock so the shape is
        // visible and caller can iterate on the UI without a backend.
        // Defer via microtask so setState doesn't fire synchronously
        // inside the caller's effect body.
        await Promise.resolve()
        if (signal?.aborted) return
        setData(MOCK_RESPONSE)
        setUsingMock(true)
        setError(null)
        setLoading(false)
        return
      }
      const delays = [0, 200, 500]
      let lastErr: unknown = null
      for (const delay of delays) {
        if (signal?.aborted) return
        if (delay > 0) {
          await new Promise<void>((resolve) => setTimeout(resolve, delay))
          if (signal?.aborted) return
        }
        const fetchResult = await getBoard(
          { apiBase: bridge.apiBase, authToken: bridge.authToken },
          signal
        ).value
        if (fetchResult.kind === 'success') {
          setData(fetchResult.value)
          setUsingMock(false)
          setError(null)
          setLoading(false)
          return
        }
        if (signal?.aborted) return
        lastErr = new Error(describeHttpError(fetchResult.reason))
      }
      console.error('bpmux board: fetch failed, falling back to mock', lastErr)
      setData((prev) => prev ?? MOCK_RESPONSE)
      setUsingMock((prev) => prev || !prev)
      setError(lastErr instanceof Error ? lastErr.message : String(lastErr))
      setLoading(false)
    },
    [bridge?.apiBase, bridge?.authToken]
  )

  // Live-refresh (#353): Board subscribes to BOARD_INVALIDATE_MARKER
  // broadcasts on the chat stream — same pattern as DrillIn's comment
  // refresh (#345). Ref tracks last-seen count so backfill markers on
  // mount don't trigger a refetch (the initial fetch below handles
  // first load).
  const seenMessageCountRef = useRef<number>(messages.length)

  useEffect(() => {
    const controller = new AbortController()
    // Initial fetch + safety-net polling (slow interval, primary
    // refresh is the invalidate subscription below). fetchBoard
    // resolves async so any setState happens after the effect body
    // returns, per React's external-data-sync pattern.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void fetchBoard(controller.signal)
    const id = setInterval(() => {
      void fetchBoard()
    }, POLL_INTERVAL_MS)
    return () => {
      controller.abort()
      clearInterval(id)
    }
  }, [fetchBoard])

  useRefetchOnReconnect(wsStatus ?? 'disconnected', () => {
    void fetchBoard()
  })

  // Invalidate subscription: any new BOARD_INVALIDATE_MARKER on the
  // chat stream triggers a refetch. Same ref-tracked last-seen-count
  // gating as DrillIn so backfill on initial hydration doesn't thrash.
  useEffect(() => {
    const prev = seenMessageCountRef.current
    if (messages.length <= prev) {
      seenMessageCountRef.current = messages.length
      return
    }
    const fresh = messages.slice(prev)
    seenMessageCountRef.current = messages.length
    if (fresh.some((m) => m.text === BOARD_INVALIDATE_MARKER)) {
      void fetchBoard()
    }
  }, [messages, fetchBoard])

  const stories = useMemo(() => data?.stories ?? [], [data])
  const epics = useMemo(() => stories.filter(isActiveEpic), [stories])
  const epicTitleByNumber = useMemo(() => {
    const map = new Map<number, string>()
    for (const s of stories) {
      if (storyTypeFromLabels(s.labels) === 'epic') map.set(s.number, s.title)
    }
    return map
  }, [stories])
  const nonEpicStories = useMemo(
    () => stories.filter((s) => storyTypeFromLabels(s.labels) !== 'epic'),
    [stories]
  )
  const iterationContainerStories = useMemo(
    () =>
      nonEpicStories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return true
        if (s.state === 'icebox') return true
        if (hasIterationMemberLabel(s.labels)) return true
        return false
      }),
    [nonEpicStories]
  )
  const flatBoardStories = useMemo(
    () =>
      nonEpicStories.filter((s) => {
        if (storyTypeFromLabels(s.labels) === 'iteration') return false
        if (s.state === 'icebox') return false
        return true
      }),
    [nonEpicStories]
  )
  const grouped = useMemo(() => groupStories(flatBoardStories), [flatBoardStories])

  // Flat navigation order matches the visible list order: for each
  // group in GROUP_ORDER, the group's stories in array order. We use
  // this for arrow-key navigation.
  const navOrder = useMemo(() => {
    const out: number[] = []
    for (const group of GROUP_ORDER) {
      const list = grouped.get(group.state) ?? []
      for (const s of list) out.push(s.number)
    }
    return out
  }, [grouped])

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
  const lastDrilledStoryRef = useRef<typeof drilledStory>(null)
  if (drilledStory !== null) {
    lastDrilledStoryRef.current = drilledStory
  }
  const drillAnim = useAnimatedUnmount(drilledStory !== null, 250)
  const renderedDrill = drilledStory ?? lastDrilledStoryRef.current

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
      tabIndex={0}
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
          <button
            type="button"
            className="board-error-retry"
            onClick={() => {
              void fetchBoard()
            }}
          >
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
