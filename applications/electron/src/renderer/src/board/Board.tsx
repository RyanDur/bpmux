import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useAnimatedUnmount } from '../hooks/useAnimatedUnmount'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import DrillIn from './DrillIn'
import StoryBadge, { deriveBadge } from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import {
  GROUP_ORDER,
  storyTypeFromLabels,
  type BoardResponse,
  type BoardStory,
  type GroupSpec,
  type StoryState
} from './types'
import './Board.css'

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

interface BoardProps {
  // Chat message stream — forwarded to DrillIn so it can subscribe to
  // BOARD_INVALIDATE_MARKER broadcasts for live comment refresh.
  // Board itself doesn't observe messages; Board polls /api/board on
  // a 30s interval for story list changes, which is orthogonal.
  messages: ChatMessage[]
  // External drill-in request (e.g., click on a search result). When
  // set, Board opens the drill-in for that story number. Null = no
  // external request; Board uses its internal selection state. The
  // commentId field (optional) is forwarded to DrillIn for scroll-to-
  // comment after the hit from the Comments search section.
  drillRequest?: { storyNumber: number; commentId?: number } | null
  // Callback fired when Board's drill-in closes (internal or external).
  // AppShell clears its drillRequest so a future re-click re-triggers.
  onDrillClosed?: () => void
  // Collapsed state for Ctrl+P toggle. When true, the aside stays
  // mounted (state preserved) but animates to 0 width via CSS. When
  // false, the aside is visible at its normal width. See Board.css
  // for the transition declaration (#351 AC3).
  hidden?: boolean
}

function Board({ messages, drillRequest, onDrillClosed, hidden }: BoardProps): React.JSX.Element {
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
      try {
        const res = await fetch(`${bridge.apiBase}/api/board`, {
          headers: { Authorization: `Bearer ${bridge.authToken}` },
          signal
        })
        if (!res.ok) {
          throw new Error(`board fetch ${res.status}`)
        }
        const json = (await res.json()) as BoardResponse
        setData(json)
        setUsingMock(false)
        setError(null)
      } catch (err) {
        if (signal?.aborted) return
        console.error('bpmux board: fetch failed, falling back to mock', err)
        // Keep last known data if we had any; otherwise show mock so
        // the panel isn't empty while the user retries.
        setData((prev) => prev ?? MOCK_RESPONSE)
        setUsingMock((prev) => prev || !prev)
        setError(err instanceof Error ? err.message : String(err))
      } finally {
        setLoading(false)
      }
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
  const grouped = useMemo(() => groupStories(stories), [stories])

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
    onDrillClosed?.()
    // Refocus the board so arrow nav resumes immediately.
    requestAnimationFrame(() => asideRef.current?.focus())
  }, [onDrillClosed])

  // External drill-request (from search result click). When the
  // prop changes to a non-null value, open drill-in. Clearing is
  // driven by the close handler above.
  useEffect(() => {
    if (drillRequest !== null && drillRequest !== undefined) {
      setSelectedNumber(drillRequest.storyNumber)
      setDrilledNumber(drillRequest.storyNumber)
    }
  }, [drillRequest])

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

  const handleReorder = useCallback(
    (state: StoryState, moved: number, insertAt: number): void => {
      const prevData = data
      // #357 rework: compute newPosition + groupSize synchronously
      // from the current `data` snapshot instead of mutating locals
      // inside the setData updater. The updater runs during React's
      // commit phase, not during the event handler — so the prior
      // code's `if (groupSize > 0)` check ran against 0 and the
      // announce never fired. Keep the optimistic reorder inside
      // setData (pure derivation from prev state), but derive the
      // announce-values up front.
      let newPosition = 0
      let groupSize = 0
      if (prevData) {
        const inGroup = prevData.stories.filter((s) => s.state === state)
        const fromIdx = inGroup.findIndex((s) => s.number === moved)
        if (fromIdx !== -1) {
          const clampedInsert = Math.max(
            0,
            Math.min(insertAt - (insertAt > fromIdx ? 1 : 0), inGroup.length - 1)
          )
          newPosition = clampedInsert + 1
          groupSize = inGroup.length
        }
      }
      setData((prev) => {
        if (!prev) return prev
        const inGroup = prev.stories.filter((s) => s.state === state)
        const others = prev.stories.filter((s) => s.state !== state)
        const fromIdx = inGroup.findIndex((s) => s.number === moved)
        if (fromIdx === -1) return prev
        const [movedStory] = inGroup.splice(fromIdx, 1)
        const clampedInsert = Math.max(
          0,
          Math.min(insertAt - (insertAt > fromIdx ? 1 : 0), inGroup.length)
        )
        inGroup.splice(clampedInsert, 0, movedStory)
        return { ...prev, stories: [...others, ...inGroup] }
      })
      if (groupSize > 0) {
        // Live-region announcement for SR users (#357 AC5).
        setReorderAnnounce(`Moved to position ${newPosition} of ${groupSize}`)
      }
      if (usingMock || !bridge?.apiBase) return
      const targetPosition = Math.max(0, insertAt)
      fetch(`${bridge.apiBase}/api/stories/${moved}/reorder`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${bridge.authToken ?? ''}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ state, position: targetPosition })
      })
        .then((res) => {
          if (!res.ok) {
            setData(prevData)
            setError(`Reorder failed: HTTP ${res.status}`)
          }
        })
        .catch((err) => {
          setData(prevData)
          setError(`Reorder failed: ${err instanceof Error ? err.message : String(err)}`)
        })
    },
    [bridge?.apiBase, bridge?.authToken, data, usingMock]
  )

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
      className={`board${hidden ? ' board--hidden' : ''}`}
      data-test="board-region"
      tabIndex={0}
      ref={asideRef}
      onKeyDown={handleKeyDown}
      aria-label="Board"
      aria-hidden={hidden ? true : undefined}
      // inert removes the element from focus + pointer events when
      // animating out; aria-hidden keeps AT from traversing.
      inert={hidden ? true : undefined}
    >
      <header className="board-header">
        <h2 className="board-title">Board</h2>
        {usingMock ? (
          <small className="board-mock-flag" aria-label="showing mock data">
            mock
          </small>
        ) : null}
      </header>
      {/* Reorder live-region (#357 AC5). Visually hidden; SR users
        * hear the new position after each keyboard reorder. aria-live
        * polite queues without interrupting the current announcement
        * (e.g., the aria-label the focused story button just
        * updated to). */}
      <div
        className="sr-only"
        role="status"
        aria-live="polite"
        data-test="board-reorder-announce"
      >
        {reorderAnnounce}
      </div>

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
          scrollToCommentId={
            drillRequest?.storyNumber === renderedDrill.number
              ? drillRequest.commentId ?? null
              : null
          }
          focusTimeline={focusTimelineFor === renderedDrill.number}
          onBack={handleBack}
          onStoryMutated={handleStoryMutated}
        />
      ) : (
        <section className="board-body" data-test="board-list">
          {GROUP_ORDER.map((group) => (
            <BoardGroup
              key={group.state}
              group={group}
              stories={grouped.get(group.state) ?? []}
              loading={loading && !data}
              selectedNumber={effectiveSelected}
              onOpen={handleOpen}
              onOpenTimeline={handleOpenTimeline}
              onSelect={setSelectedNumber}
              onReorder={handleReorder}
            />
          ))}
        </section>
      )}
    </aside>
  )
}

function groupStories(stories: BoardStory[]): Map<StoryState, BoardStory[]> {
  const out = new Map<StoryState, BoardStory[]>()
  for (const group of GROUP_ORDER) out.set(group.state, [])
  for (const story of stories) {
    const list = out.get(story.state)
    if (list) list.push(story)
  }
  return out
}

interface GroupProps {
  group: GroupSpec
  stories: BoardStory[]
  loading: boolean
  selectedNumber: number | null
  onOpen: (n: number) => void
  // #368: badge click path. Separate from onOpen so the row handler
  // can distinguish a plain-open from a timeline-focused-open and
  // pass the state through to DrillIn.
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  onReorder?: (state: StoryState, moved: number, insertAt: number) => void
}

function BoardGroup({
  group,
  stories,
  loading,
  selectedNumber,
  onOpen,
  onOpenTimeline,
  onSelect,
  onReorder
}: GroupProps): React.JSX.Element {
  const defaultOpen = !group.collapsed
  const draggable = group.state === 'ready' && onReorder !== undefined
  const [dropAt, setDropAt] = useState<number | null>(null)
  // #333: track the row being dragged so we can hide it via CSS
  // (opacity: 0) for the duration of the gesture. Ryan 2026-04-22
  // ~22:32 UTC: "hide the dragged item. Until not being dragged." —
  // the original ghost + insertion indicator overlap was confusing;
  // hiding the origin row makes the drop target read unambiguously.
  const [draggingNumber, setDraggingNumber] = useState<number | null>(null)

  const handleListDragOver = (event: React.DragEvent<HTMLUListElement>): void => {
    if (!draggable) return
    event.preventDefault()
    event.dataTransfer.dropEffect = 'move'
  }

  const handleRowDragOver = (index: number) => (event: React.DragEvent<HTMLLIElement>): void => {
    if (!draggable) return
    event.preventDefault()
    const rect = event.currentTarget.getBoundingClientRect()
    const mid = rect.top + rect.height / 2
    setDropAt(event.clientY < mid ? index : index + 1)
  }

  const handleDrop = (event: React.DragEvent<HTMLUListElement>): void => {
    if (!draggable || onReorder === undefined) return
    event.preventDefault()
    const raw = event.dataTransfer.getData('text/bpmux-story')
    const moved = Number.parseInt(raw, 10)
    if (Number.isNaN(moved) || dropAt === null) {
      setDropAt(null)
      setDraggingNumber(null)
      return
    }
    onReorder(group.state, moved, dropAt)
    setDropAt(null)
    setDraggingNumber(null)
  }

  return (
    <details className="board-group" open={defaultOpen} data-test={`board-group-${group.state}`}>
      <summary className="board-group-summary">
        <b className="board-group-label">{group.label}</b>
        <small className="board-group-count">{stories.length}</small>
      </summary>
      {loading ? (
        <ul className="board-story-list board-story-list--skeleton" aria-hidden="true">
          {[0, 1, 2].map((i) => (
            <li key={i} className="board-story-row board-story-row--skeleton">
              <i className="skeleton-bar" />
            </li>
          ))}
        </ul>
      ) : stories.length === 0 ? (
        <p className="board-group-empty">none</p>
      ) : (
        <ul
          className="board-story-list"
          onDragOver={handleListDragOver}
          onDrop={handleDrop}
          onDragLeave={() => setDropAt(null)}
        >
          {stories.map((story, i) => (
            <StoryRow
              key={story.number}
              story={story}
              selected={story.number === selectedNumber}
              draggable={draggable}
              dragging={draggingNumber === story.number}
              dropBefore={dropAt === i}
              dropAfterLast={dropAt === stories.length && i === stories.length - 1}
              position={i + 1}
              total={stories.length}
              onKeyboardReorder={
                draggable && onReorder !== undefined
                  ? (direction) => {
                      // Compute insertAt for the handleReorder
                      // contract: passed index is the insertion
                      // point in the PRE-removal list.
                      //   up: insertAt = fromIdx - 1 (clamp >=0)
                      //   down: insertAt = fromIdx + 2 (clamp <=len)
                      // clampedInsert in handleReorder further
                      // normalises once the moved item is removed,
                      // so this matches the drag-drop semantics.
                      if (direction === 'up' && i === 0) return
                      if (direction === 'down' && i === stories.length - 1) return
                      const insertAt =
                        direction === 'up'
                          ? Math.max(0, i - 1)
                          : Math.min(stories.length, i + 2)
                      onReorder(group.state, story.number, insertAt)
                    }
                  : undefined
              }
              onOpen={onOpen}
              onOpenTimeline={onOpenTimeline}
              onSelect={onSelect}
              onRowDragOver={handleRowDragOver(i)}
              onDragStartRow={() => setDraggingNumber(story.number)}
              onDragEndRow={() => {
                setDraggingNumber(null)
                setDropAt(null)
              }}
            />
          ))}
        </ul>
      )}
    </details>
  )
}

interface RowProps {
  story: BoardStory
  selected: boolean
  draggable: boolean
  // #333: parent tracks which row is being dragged; the matching
  // row hides via [data-dragging="true"] CSS for the duration of
  // the gesture. Reappears at the drop position when React re-
  // renders with the new order.
  dragging: boolean
  dropBefore: boolean
  dropAfterLast: boolean
  // Keyboard-reorder metadata (#357). position is 1-based within
  // the current group; total is the group's size. Both flow into the
  // aria-label so screen readers announce where the story sits.
  // onKeyboardReorder fires Ctrl+Shift+ArrowUp / ArrowDown intents.
  position: number
  total: number
  onKeyboardReorder?: (direction: 'up' | 'down') => void
  onOpen: (n: number) => void
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  onRowDragOver: (event: React.DragEvent<HTMLLIElement>) => void
  // #333: signal to the parent that this row has started / ended a
  // drag gesture. Parent uses the signal to set + clear the
  // hide-while-dragging state.
  onDragStartRow?: () => void
  onDragEndRow?: () => void
}

function StoryRow({
  story,
  selected,
  draggable,
  dragging,
  dropBefore,
  dropAfterLast,
  position,
  total,
  onKeyboardReorder,
  onOpen,
  onOpenTimeline,
  onSelect,
  onRowDragOver,
  onDragStartRow,
  onDragEndRow
}: RowProps): React.JSX.Element {
  const type = storyTypeFromLabels(story.labels)
  const classes = ['board-story-row']
  if (dropBefore) classes.push('board-story-row--drop-before')
  if (dropAfterLast) classes.push('board-story-row--drop-after')
  // #390: dim the row body on blocked-in-delivered so the
  // acceptance walk reads blocked stories as "not in your queue
  // right now" at a glance. Row stays visible + readable (nothing-
  // hidden); opacity is the secondary axis, the badge + glyph carry
  // the primary signal.
  if (deriveBadge(story) === 'blocked') classes.push('board-story-row--blocked')
  // aria-label describes position + reorder affordance for screen-
  // reader users; keyboard-only users read the same hint in the
  // button's tooltip on focus.
  const reorderHint =
    onKeyboardReorder !== undefined
      ? ' — press Ctrl+Shift+Up to move up, Ctrl+Shift+Down to move down'
      : ''
  const ariaLabel = `Story #${story.number} ${story.title}, position ${position} of ${total}${reorderHint}`
  return (
    <li
      className={classes.join(' ')}
      aria-current={selected ? 'true' : undefined}
      data-test={`board-story-${story.number}`}
      data-dragging={dragging ? 'true' : undefined}
      draggable={draggable}
      onDragStart={(event) => {
        if (!draggable) return
        event.dataTransfer.setData('text/bpmux-story', String(story.number))
        event.dataTransfer.effectAllowed = 'move'
        onDragStartRow?.()
      }}
      onDragEnd={() => onDragEndRow?.()}
      onDragOver={onRowDragOver}
    >
      <button
        type="button"
        className="board-story-button"
        onClick={() => onOpen(story.number)}
        onFocus={() => onSelect(story.number)}
        onMouseEnter={() => onSelect(story.number)}
        onKeyDown={(event) => {
          if (onKeyboardReorder === undefined) return
          if (!event.ctrlKey || !event.shiftKey) return
          if (event.key === 'ArrowUp') {
            event.preventDefault()
            onKeyboardReorder('up')
          } else if (event.key === 'ArrowDown') {
            event.preventDefault()
            onKeyboardReorder('down')
          }
        }}
        title={story.title}
        aria-label={ariaLabel}
      >
        <small className="board-story-number">#{story.number}</small>
        <StoryTypeGlyph type={type} />
        <StoryBadge story={story} onOpenTimeline={onOpenTimeline} onOpenBlocker={onOpen} />
        <b className="board-story-title">{story.title}</b>
      </button>
    </li>
  )
}

export default Board
