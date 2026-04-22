import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { ChatMessage } from '../transport/types'
import { RENDERER_IDENTITY } from '../transport/useWebSocket'
import DrillIn from './DrillIn'
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

const POLL_INTERVAL_MS = 30_000

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
}

function Board({ messages }: BoardProps): React.JSX.Element {
  const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
  const [data, setData] = useState<BoardResponse | null>(null)
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<string | null>(null)
  const [usingMock, setUsingMock] = useState<boolean>(false)
  const [selectedNumber, setSelectedNumber] = useState<number | null>(null)
  const [drilledNumber, setDrilledNumber] = useState<number | null>(null)

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

  useEffect(() => {
    const controller = new AbortController()
    // Initial fetch + 30s polling is external-data sync (the React-
    // recommended effect use); fetchBoard resolves asynchronously so
    // any setState happens after the effect body returns.
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

  const handleOpen = useCallback((number: number) => {
    setSelectedNumber(number)
    setDrilledNumber(number)
  }, [])

  const handleBack = useCallback(() => {
    setDrilledNumber(null)
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

  const handleReorder = useCallback(
    (state: StoryState, moved: number, insertAt: number): void => {
      const prevData = data
      setData((prev) => {
        if (!prev) return prev
        const inGroup = prev.stories.filter((s) => s.state === state)
        const others = prev.stories.filter((s) => s.state !== state)
        const fromIdx = inGroup.findIndex((s) => s.number === moved)
        if (fromIdx === -1) return prev
        const [movedStory] = inGroup.splice(fromIdx, 1)
        const clampedInsert = Math.max(0, Math.min(insertAt - (insertAt > fromIdx ? 1 : 0), inGroup.length))
        inGroup.splice(clampedInsert, 0, movedStory)
        return { ...prev, stories: [...others, ...inGroup] }
      })
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
      className="board"
      data-test="board-region"
      tabIndex={0}
      ref={asideRef}
      onKeyDown={handleKeyDown}
      aria-label="Board"
    >
      <header className="board-header">
        <h2 className="board-title">Board</h2>
        {usingMock ? (
          <small className="board-mock-flag" aria-label="showing mock data">
            mock
          </small>
        ) : null}
      </header>

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

      {drilledStory !== null ? (
        <DrillIn
          story={drilledStory}
          isOwner={isOwner}
          apiBase={bridge?.apiBase}
          authToken={bridge?.authToken}
          usingMock={usingMock}
          messages={messages}
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
  onSelect: (n: number) => void
  onReorder?: (state: StoryState, moved: number, insertAt: number) => void
}

function BoardGroup({
  group,
  stories,
  loading,
  selectedNumber,
  onOpen,
  onSelect,
  onReorder
}: GroupProps): React.JSX.Element {
  const defaultOpen = !group.collapsed
  const draggable = group.state === 'ready' && onReorder !== undefined
  const [dropAt, setDropAt] = useState<number | null>(null)

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
      return
    }
    onReorder(group.state, moved, dropAt)
    setDropAt(null)
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
              dropBefore={dropAt === i}
              dropAfterLast={dropAt === stories.length && i === stories.length - 1}
              onOpen={onOpen}
              onSelect={onSelect}
              onRowDragOver={handleRowDragOver(i)}
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
  dropBefore: boolean
  dropAfterLast: boolean
  onOpen: (n: number) => void
  onSelect: (n: number) => void
  onRowDragOver: (event: React.DragEvent<HTMLLIElement>) => void
}

function StoryRow({
  story,
  selected,
  draggable,
  dropBefore,
  dropAfterLast,
  onOpen,
  onSelect,
  onRowDragOver
}: RowProps): React.JSX.Element {
  const type = storyTypeFromLabels(story.labels)
  const classes = ['board-story-row']
  if (dropBefore) classes.push('board-story-row--drop-before')
  if (dropAfterLast) classes.push('board-story-row--drop-after')
  return (
    <li
      className={classes.join(' ')}
      aria-current={selected ? 'true' : undefined}
      data-test={`board-story-${story.number}`}
      draggable={draggable}
      onDragStart={(event) => {
        if (!draggable) return
        event.dataTransfer.setData('text/bpmux-story', String(story.number))
        event.dataTransfer.effectAllowed = 'move'
      }}
      onDragOver={onRowDragOver}
    >
      <button
        type="button"
        className="board-story-button"
        onClick={() => onOpen(story.number)}
        onFocus={() => onSelect(story.number)}
        onMouseEnter={() => onSelect(story.number)}
        title={story.title}
      >
        <small className="board-story-number">#{story.number}</small>
        <StoryTypeGlyph type={type} />
        <b className="board-story-title">{story.title}</b>
      </button>
    </li>
  )
}

export default Board
