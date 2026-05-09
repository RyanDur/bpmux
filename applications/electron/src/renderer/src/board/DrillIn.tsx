import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import MessageBody from '../chat/MessageBody'
import {
  useTimeline,
  type ClassifiedEvent,
  type TimelineGate
} from '../hooks/useTimeline'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'
import StoryBadge from './StoryBadge'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyTypeFromLabels, type BoardStory } from './types'
import './DrillIn.css'

const LIFECYCLE_LABELS: ReadonlySet<string> = new Set([
  'delivered',
  'ready for accept',
  'ready-for-accept',
  'desk-check-fail',
  'desk-check-pass',
  'blocked',
  'rework',
  'rejected',
  'ready'
])

const isLifecycleLabel = (label: string): boolean =>
  LIFECYCLE_LABELS.has(label) || label.startsWith('blocked-by:')

// Human-readable label per lifecycle gate. Keeps the rendering code
// in DrillIn concise — map lives at module scope so the object
// doesn't re-create per render.
const GATE_LABEL: Record<TimelineGate, string> = {
  filed: 'Filed',
  started: 'Started',
  delivered: 'Delivered',
  'desk-check-fail': 'Desk-check failed',
  accepted: 'Accepted',
  rejected: 'Rejected',
  closed: 'Closed',
  reopened: 'Reopened'
}

// "Major gates" set — timeline default-filter. Matches the AC's
// filing / started / delivered / desk-check pass|fail / accepted |
// rejected / closed list. (desk-check-pass isn't a distinct label in
// our current workflow — successful desk-check flows straight into
// accepted; the `accepted` gate stands in.)
const MAJOR_GATES: ReadonlySet<TimelineGate> = new Set([
  'filed',
  'started',
  'delivered',
  'desk-check-fail',
  'accepted',
  'rejected',
  'closed',
  'reopened'
])

// Stories filed before this date pre-date the bpmux lifecycle-label
// vocabulary being in force. We footer-note them so the reader
// understands why their timeline is sparse compared to newer stories.
// Mirrors the AC's "pre-event-log" framing. Concrete cutoff chosen
// as 2026-04-01 per the workflow rollout window.
const EVENT_LOG_START = Date.parse('2026-04-01T00:00:00Z')

const labelFor = (e: ClassifiedEvent): string => {
  if (e.gate) return GATE_LABEL[e.gate]
  if (e.raw.event === 'labeled') return `Labeled: ${e.raw.label ?? '?'}`
  if (e.raw.event === 'unlabeled') return `Unlabeled: ${e.raw.label ?? '?'}`
  if (e.raw.event === 'commented') return 'Commented'
  if (e.raw.event === 'renamed') return 'Renamed'
  if (e.raw.event === 'assigned') return 'Assigned'
  if (e.raw.event === 'unassigned') return 'Unassigned'
  return e.raw.event
}

const truncate = (body: string, max = 120): string => {
  const collapsed = body.replace(/\s+/g, ' ').trim()
  if (collapsed.length <= max) return collapsed
  return `${collapsed.slice(0, max - 1)}…`
}

const renderTimelineList = (
  events: ClassifiedEvent[],
  showAll: boolean,
  story: BoardStory
): React.JSX.Element => {
  const visible = showAll
    ? events
    : events.filter((e) => e.gate !== null && MAJOR_GATES.has(e.gate))
  const createdMs = Date.parse(story.created_at ?? '')
  const preEventLog =
    Number.isFinite(createdMs) && createdMs < EVENT_LOG_START
  return (
    <>
      <ol className="drill-in-timeline-list" data-test="drill-in-timeline-list">
        {visible.map((e, i) => {
          const classes = [
            'drill-in-timeline-item',
            e.gate ? `drill-in-timeline-item--${e.gate}` : '',
            e.reworkGroup !== undefined
              ? 'drill-in-timeline-item--rework'
              : ''
          ]
            .filter(Boolean)
            .join(' ')
          return (
            <li
              key={`${e.raw.ts}-${i}`}
              className={classes}
              data-test="drill-in-timeline-item"
              data-gate={e.gate ?? e.raw.event}
              data-rework-group={e.reworkGroup ?? undefined}
            >
              <span className="drill-in-timeline-kind">{labelFor(e)}</span>
              <time
                className="drill-in-timeline-ts"
                dateTime={e.raw.ts}
              >
                {format(new Date(e.raw.ts), 'MMM d · h:mm a')}
              </time>
              {e.raw.actor && (
                <cite className="drill-in-timeline-actor">{e.raw.actor}</cite>
              )}
              {e.raw.body && (
                <small className="drill-in-timeline-note">
                  {truncate(e.raw.body)}
                </small>
              )}
            </li>
          )
        })}
      </ol>
      {preEventLog && (
        <p
          className="drill-in-timeline-footer"
          data-test="drill-in-timeline-prelog"
        >
          history pre-dates event log
        </p>
      )}
    </>
  )
}

interface Comment {
  id: number
  author: string
  body: string
  created_at: string
}

// Optimistic comments have a placeholder id in this range so the dedup
// step can tell them apart from server-assigned ids (which are GitHub
// issue comment ids, always positive). When the refetch (after the
// invalidate broadcast) returns the real comment with a real id, the
// optimistic one is dropped by the merge in mergeComments().
const OPTIMISTIC_ID_BASE = -1_000_000_000

interface Props {
  story: BoardStory
  isOwner: boolean
  apiBase: string | undefined
  authToken: string | undefined
  usingMock: boolean
  // Chat message stream from the WS. DrillIn watches this for
  // BOARD_INVALIDATE_MARKER messages (published when any peer posts
  // a comment, accepts/rejects, etc.) and refetches comments live —
  // the viewer sees peer activity without having to exit + re-drill.
  messages: ChatMessage[]
  // #354 close-direction: Board's useAnimatedUnmount flips this to
  // false to trigger the exit transition before unmount.
  visible: boolean
  // #368: when true, DrillIn auto-expands the timeline section + scrolls
  // it into view on mount. Used by the team-accept badge click path so
  // opening a drill-in via the badge lands directly on the audit trail.
  focusTimeline?: boolean
  onBack: () => void
  onStoryMutated: (story: BoardStory) => void
}

const isInvalidateMarker = (m: ChatMessage): boolean => m.text === BOARD_INVALIDATE_MARKER

const mergeComments = (
  server: readonly Comment[],
  optimistic: readonly Comment[]
): Comment[] => {
  if (optimistic.length === 0) return [...server]
  const survivors = optimistic.filter((opt) => {
    const optMs = new Date(opt.created_at).getTime()
    return !server.some((s) => {
      if (s.author !== opt.author) return false
      if (s.body !== opt.body) return false
      const sMs = new Date(s.created_at).getTime()
      return Math.abs(sMs - optMs) < 10_000
    })
  })
  return [...server, ...survivors]
}

type ActionMode = 'idle' | 'bug-question' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

const TOAST_MS = 4000

const DrillIn = ({
  story,
  isOwner,
  apiBase,
  authToken,
  usingMock,
  messages,
  visible,
  focusTimeline,
  onBack,
  onStoryMutated
}: Props): React.JSX.Element => {
  const [mode, setMode] = useState<ActionMode>('idle')
  const [busy, setBusy] = useState<BusyKind>(null)
  const [note, setNote] = useState<string>('')
  const [bypassReason, setBypassReason] = useState<string>('')
  const [toast, setToast] = useState<string | null>(null)
  const [serverComments, setServerComments] = useState<Comment[] | null>(null)
  const [optimistic, setOptimistic] = useState<Comment[]>([])
  const [commentDraft, setCommentDraft] = useState<string>('')
  const [commentBusy, setCommentBusy] = useState<boolean>(false)
  // #367 timeline filter state. Default: major gates only, matching
  // the spec's "don't overwhelm" framing. Toggled via the "show all"
  // button below the timeline header — persistent across re-renders
  // of the same drill-in instance.
  const [timelineShowAll, setTimelineShowAll] = useState<boolean>(false)
  const timeline = useTimeline({
    apiBase,
    authToken,
    storyNumber: story.number,
    messages
  })
  const articleRef = useRef<HTMLElement | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  const rejectDialogRef = useRef<HTMLDialogElement | null>(null)
  // #368 badge-click → focus the timeline section. Ref instead of
  // a controlled `open` prop so the user can still toggle the
  // details closed after the initial focus without losing the state
  // to a re-render pulse. The effect below flips open + scrolls only
  // when `focusTimeline` transitions to true.
  const timelineDetailsRef = useRef<HTMLDetailsElement | null>(null)
  const commentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  // Invalidate subscription: remember how many messages we've seen so
  // we only react to NEW marker messages (not the backfill hydration
  // on mount). Tracked as a ref so the effect doesn't depend on
  // previous message counts (which would re-trigger needlessly).
  const seenMessageCountRef = useRef<number>(messages.length)
  // Scroll preservation: track whether the comments list was scrolled
  // to bottom BEFORE a refetch; after rerender, if so, auto-scroll to
  // new bottom. If not, preserve scroll position so the reader isn't
  // yanked away from mid-thread content.
  const commentsListRef = useRef<HTMLUListElement | null>(null)
  const wasAtBottomRef = useRef<boolean>(true)

  const comments = serverComments === null ? null : mergeComments(serverComments, optimistic)


  // Focus the article on mount so Esc falls to our handler before
  // anything upstream intercepts.
  useEffect(() => {
    articleRef.current?.focus()
  }, [])

  useEffect(() => {
    const dialog = rejectDialogRef.current
    if (!dialog) return
    if (mode === 'rejecting') {
      if (!dialog.open) {
        dialog.showModal()
      }
      textareaRef.current?.focus()
    } else if (dialog.open) {
      dialog.close()
    }
  }, [mode])

// #368: when the drill-in is opened via a team-accept badge click,
  // auto-expand the timeline details + scroll the section into view
  // so the user lands directly on the audit trail. RAF defers the
  // DOM mutation past the mount commit so the scroll measurement
  // picks up the now-open layout.
  useEffect(() => {
    if (!focusTimeline) return
    const el = timelineDetailsRef.current
    if (!el) return
    el.open = true
    window.requestAnimationFrame(() => {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    })
  }, [focusTimeline])

  useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), TOAST_MS)
    return () => clearTimeout(id)
  }, [toast])

  // Capture scroll state BEFORE the fetch so post-render can restore.
  const captureScroll = useCallback((): void => {
    const el = commentsListRef.current
    if (!el) return
    // "At bottom" tolerance: treat within 4px of the max scrollTop as
    // "at bottom" — scroll rounding in Chromium can leave 1-2px gap.
    const max = el.scrollHeight - el.clientHeight
    wasAtBottomRef.current = max <= 0 || el.scrollTop >= max - 4
  }, [])

  const fetchComments = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      captureScroll()
      if (usingMock || !apiBase) {
        setServerComments([])
        return
      }
      try {
        const res = await fetch(
          `${apiBase}/api/stories/${story.number}/comments`,
          {
            headers: { Authorization: `Bearer ${authToken ?? ''}` },
            signal
          }
        )
        if (!res.ok) {
          setServerComments([])
          return
        }
        const json = (await res.json()) as Comment[]
        setServerComments(json)
      } catch (err) {
        if ((err as Error)?.name === 'AbortError') return
        setServerComments([])
      }
    },
    [apiBase, authToken, captureScroll, story.number, usingMock]
  )

  useEffect(() => {
    const controller = new AbortController()
    void fetchComments(controller.signal)
    return () => controller.abort()
  }, [fetchComments])

  // Live-refresh: watch for new BOARD_INVALIDATE_MARKER messages on
  // the chat stream. On a new marker, refetch comments. The ref tracks
  // last-seen message count so we only react to freshly arrived
  // markers, not the backfill on initial mount (which often contains
  // old invalidate markers from earlier in the session).
  useEffect(() => {
    const prev = seenMessageCountRef.current
    if (messages.length <= prev) {
      seenMessageCountRef.current = messages.length
      return
    }
    const fresh = messages.slice(prev)
    seenMessageCountRef.current = messages.length
    if (fresh.some(isInvalidateMarker)) {
      void fetchComments()
    }
  }, [messages, fetchComments])

  // Post-render scroll restoration: if viewer was at bottom before
  // the refetch, auto-scroll to bottom so they see the new entries.
  // Otherwise the browser preserves scrollTop naturally (React keeps
  // the scroll container in place across a list re-render with stable
  // keys), so there's nothing to restore — they stay where they were.
  useEffect(() => {
    if (!wasAtBottomRef.current) return
    const el = commentsListRef.current
    if (!el) return
    el.scrollTop = el.scrollHeight
  }, [comments])

  const handleCommentSubmit = useCallback(async () => {
    const trimmed = commentDraft.trim()
    if (!trimmed || commentBusy) return
    setCommentBusy(true)
    // Optimistic: show the comment immediately so the user sees their
    // message land without waiting for the POST roundtrip + invalidate
    // refetch cycle. Negative id marks it as optimistic for the merge
    // dedup. If the POST fails we roll it back via the catch below.
    const optimisticEntry: Comment = {
      id: OPTIMISTIC_ID_BASE - Date.now(),
      author: 'ryan',
      body: trimmed,
      created_at: new Date().toISOString()
    }
    setOptimistic((prev) => [...prev, optimisticEntry])
    setCommentDraft('')
    if (usingMock || !apiBase) {
      // Mock path: no server, no invalidate — the optimistic entry
      // just stays (it's the only record of the comment).
      setCommentBusy(false)
      return
    }
    try {
      const res = await fetch(`${apiBase}/api/stories/${story.number}/comments`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${authToken ?? ''}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ body: trimmed })
      })
      if (!res.ok) {
        // Roll back the optimistic entry + restore the draft so the
        // user can retry. Otherwise the comment silently disappears
        // and they have to retype.
        setOptimistic((prev) => prev.filter((c) => c.id !== optimisticEntry.id))
        setCommentDraft(trimmed)
        setToast(`Comment failed: HTTP ${res.status}`)
      }
      // Success: the server will broadcast BOARD_INVALIDATE_MARKER.
      // The invalidate-watcher effect above refetches, mergeComments
      // replaces the optimistic entry with the server-authoritative
      // one (matched by body + author + ≤10s created_at).
    } catch (err) {
      setOptimistic((prev) => prev.filter((c) => c.id !== optimisticEntry.id))
      setCommentDraft(trimmed)
      setToast(`Comment failed: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setCommentBusy(false)
    }
  }, [apiBase, authToken, commentBusy, commentDraft, story.number, usingMock])

  const canAct = isOwner && story.state === 'delivered'

  const fireAccept = useCallback(
    async (payload: {
      would_file_bug: boolean
      force?: boolean
      bypass_reason?: string
    }): Promise<void> => {
      if (busy !== null) return
      setBusy('accept')
      const rollback = (reason: string): void => {
        setBusy(null)
        setToast(`Accept failed: ${reason}`)
      }
      try {
        if (usingMock || !apiBase) {
          onStoryMutated({ ...story, state: 'accepted' })
          setBusy(null)
          onBack()
          return
        }
        const res = await fetch(`${apiBase}/api/stories/${story.number}/accept`, {
          method: 'POST',
          headers: {
            Authorization: `Bearer ${authToken ?? ''}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(payload)
        })
        if (!res.ok) {
          const detail = await res.text().catch(() => '')
          rollback(`HTTP ${res.status}${detail ? ` — ${detail}` : ''}`)
          return
        }
        onStoryMutated({ ...story, state: 'accepted' })
        setBusy(null)
        onBack()
      } catch (err) {
        rollback(err instanceof Error ? err.message : String(err))
      }
    },
    [apiBase, authToken, busy, onBack, onStoryMutated, story, usingMock]
  )

  const handleAccept = useCallback((): void => {
    setMode('bug-question')
  }, [])

  const handleAcceptConfirm = useCallback((): void => {
    setMode('idle')
    void fireAccept({ would_file_bug: false })
  }, [fireAccept])

  const handleAcceptRouteToReject = useCallback((): void => {
    setMode('rejecting')
  }, [])

  const handleAcceptBypass = useCallback((): void => {
    const trimmed = bypassReason.trim()
    if (!trimmed) return
    setMode('idle')
    setBypassReason('')
    void fireAccept({ would_file_bug: true, force: true, bypass_reason: trimmed })
  }, [bypassReason, fireAccept])

  const handleRejectSend = useCallback(async () => {
    const trimmed = note.trim()
    if (!trimmed || busy !== null) return
    setBusy('reject')
    const rollback = (reason: string): void => {
      setBusy(null)
      setToast(`Reject failed: ${reason}`)
    }
    try {
      if (usingMock || !apiBase) {
        onStoryMutated({ ...story, state: 'ready' })
        setBusy(null)
        setMode('idle')
        setNote('')
        onBack()
        return
      }
      const res = await fetch(`${apiBase}/api/stories/${story.number}/reject`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${authToken ?? ''}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ note: trimmed })
      })
      if (!res.ok) {
        const detail = await res.text().catch(() => '')
        rollback(`HTTP ${res.status}${detail ? ` — ${detail}` : ''}`)
        return
      }
      onStoryMutated({ ...story, state: 'ready' })
      setBusy(null)
      setMode('idle')
      setNote('')
      onBack()
    } catch (err) {
      rollback(err instanceof Error ? err.message : String(err))
    }
  }, [apiBase, authToken, busy, note, onBack, onStoryMutated, story, usingMock])

  // #366 quote-excerpt: shared insertion logic. Either the keyboard
  // shortcut (Cmd/Ctrl+E) or the native right-click "Quote in reply"
  // context-menu item (#371) calls this with the selection's
  // comment id + author + text. Prepends a markdown blockquote to
  // the comment draft, then focuses the textarea with the cursor
  // placed AFTER the blank line that follows the quote block so
  // the author can type their reply immediately. The anchor in the
  // blockquote links back to the source comment via `#comment-<id>`;
  // MessageBody renders this as a clickable scroll-to-origin link.
  // Rationale: concurrent-write pain (Ryan 2026-04-22) — mid-compose,
  // new comments arrive and the reply's antecedent becomes ambiguous.
  const insertQuote = useCallback(
    (id: string, author: string, text: string) => {
      const selected = text.trim()
      if (!selected) return
      const quoted = selected
        .split('\n')
        .map((line) => `> ${line}`)
        .join('\n')
      const header = `> [@${author}](#comment-${id}):`
      const prefix = `${header}\n${quoted}\n\n`
      setCommentDraft((prev) => prefix + prev)
      const caret = prefix.length
      window.requestAnimationFrame(() => {
        const ta = commentTextareaRef.current
        if (!ta) return
        ta.focus()
        ta.setSelectionRange(caret, caret)
      })
    },
    []
  )

  // Keyboard path (#366): Cmd/Ctrl+E with a selection inside a
  // comment body fires the same `insertQuote` path as the native
  // right-click menu below.
  const handleQuoteShortcut = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (event.key !== 'e' && event.key !== 'E') return
      if (!(event.metaKey || event.ctrlKey)) return
      if (event.altKey || event.shiftKey) return
      const sel = window.getSelection()
      if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return
      const anchor = sel.anchorNode
      if (!anchor) return
      let node: Node | null = anchor
      while (node && !(node instanceof HTMLElement && node.dataset.commentId)) {
        node = node.parentNode
      }
      if (!(node instanceof HTMLElement)) return
      const id = node.dataset.commentId
      const author = node.dataset.commentAuthor ?? 'unknown'
      if (!id) return
      const selected = sel.toString().trim()
      if (!selected) return
      event.preventDefault()
      insertQuote(id, author, selected)
    },
    [insertQuote]
  )

  // Native-menu path (#371): preload exposes `onQuoteComment`, which
  // main triggers when the user clicks "Quote in reply" in the
  // native right-click context menu. Same insertQuote path as the
  // keyboard shortcut — single source of truth for the quote
  // format so the two trigger surfaces can't drift apart.
  useEffect(() => {
    const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
    if (!bridge?.onQuoteComment) return
    const unsubscribe = bridge.onQuoteComment(({ commentId, author, text }) => {
      insertQuote(commentId, author, text)
    })
    return unsubscribe
  }, [insertQuote])

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      // Quote-shortcut runs first — if it consumes the event
      // (selection inside a comment + Cmd/Ctrl+E), we stop here.
      handleQuoteShortcut(event)
      if (event.defaultPrevented) return
      if (event.key === 'Escape') {
        if (mode === 'rejecting') {
          return
        }
        event.preventDefault()
        if (mode === 'bug-question') {
          setMode('idle')
          return
        }
        onBack()
      }
    },
    [handleQuoteShortcut, mode, onBack]
  )

  const handleTextareaKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleRejectSend()
      }
    },
    [handleRejectSend]
  )

  // Cmd/Ctrl+Enter on the comment textarea submits the comment —
  // parity with the reject-form textarea above (#356). Shift+Enter
  // still inserts a newline (standard textarea; we don't intercept
  // that case). Matches the reject-form's handler exactly so the
  // user's muscle memory transfers between the two inputs.
  const handleCommentKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleCommentSubmit()
      }
    },
    [handleCommentSubmit]
  )

  const type = storyTypeFromLabels(story.labels)

  return (
    <article
      className="drill-in"
      data-visible={visible || undefined}
      aria-hidden={!visible || undefined}
      ref={articleRef}
      tabIndex={-1}
      data-test="board-drill-in"
      onKeyDown={handleKeyDown}
      aria-label={`Story ${story.number}: ${story.title}`}
    >
      <header className="drill-in-header">
        <button
          type="button"
          className="drill-in-back"
          onClick={onBack}
          aria-label="Back to board list"
        >
          <BackArrow />
          <small className="drill-in-crumb">Board · #{story.number}</small>
        </button>
      </header>

      <section className="drill-in-meta">
        <h3 className="drill-in-title">
          <StoryTypeGlyph type={type} />
          <b className="drill-in-title-text">{story.title}</b>
        </h3>
        <StoryBadge story={story} />
        {(() => {
          const meaningfulLabels = story.labels.filter((label) => !isLifecycleLabel(label))
          if (meaningfulLabels.length === 0) return null
          return (
            <ul className="drill-in-chips" data-test="drill-in-labels">
              {meaningfulLabels.map((label) => (
                <li key={label} className="drill-in-chip">
                  {label}
                </li>
              ))}
            </ul>
          )
        })()}
        {story.assignees.length > 0 && (
          <ul className="drill-in-chips" data-test="drill-in-assignees">
            {story.assignees.map((a) => (
              <li key={a} className="drill-in-chip">
                {a}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="drill-in-body" aria-label="Story body">
        {story.body ? (
          <MessageBody text={story.body} />
        ) : (
          <p className="drill-in-body-empty">(no body)</p>
        )}
      </section>

      <section className="drill-in-comments" aria-label="Comments">
        <header className="drill-in-comments-header">
          <h4>Comments{comments !== null ? ` (${comments.length})` : ''}</h4>
        </header>
        {comments === null ? (
          <p className="drill-in-comments-empty">loading…</p>
        ) : comments.length === 0 ? (
          <p className="drill-in-comments-empty">no comments yet</p>
        ) : (
          <ul className="drill-in-comments-list" ref={commentsListRef} data-test="drill-in-comments-list">
            {comments.map((c) => (
              <li
                key={c.id}
                className="drill-in-comment"
                data-test="drill-in-comment"
                data-comment-id={c.id}
                data-comment-author={c.author}
              >
                <article>
                  <header className="drill-in-comment-meta">
                    <cite>{c.author}</cite>
                    <time dateTime={c.created_at}>
                      {format(new Date(c.created_at), 'MMM d · h:mm a zzz')}
                    </time>
                  </header>
                  <section className="drill-in-comment-body">
                    <MessageBody text={c.body} />
                  </section>
                </article>
              </li>
            ))}
          </ul>
        )}
        <form
          className="drill-in-comment-form"
          onSubmit={(event) => {
            event.preventDefault()
            void handleCommentSubmit()
          }}
        >
          <label className="drill-in-comment-label">
            <small>Add a comment</small>
            <textarea
              ref={commentTextareaRef}
              value={commentDraft}
              onChange={(e) => setCommentDraft(e.target.value)}
              onKeyDown={handleCommentKeyDown}
              placeholder="Markdown supported — Cmd+Enter to send"
              rows={3}
              data-test="drill-in-comment-input"
            />
          </label>
          <menu className="drill-in-action-row">
            <li>
              <button
                type="submit"
                className="drill-in-btn drill-in-btn--primary"
                disabled={commentBusy || commentDraft.trim().length === 0}
                data-test="drill-in-comment-send"
              >
                {commentBusy ? 'Sending…' : 'Comment'}
              </button>
            </li>
          </menu>
        </form>
      </section>

      <details
        className="drill-in-timeline"
        data-test="drill-in-timeline"
        ref={timelineDetailsRef}
      >
        <summary className="drill-in-timeline-summary">
          <h4>Timeline{timeline.events ? ` (${timeline.events.length})` : ''}</h4>
        </summary>
        {timeline.loading && timeline.events === null ? (
          <p className="drill-in-timeline-empty">loading…</p>
        ) : timeline.error && timeline.events === null ? (
          <p className="drill-in-timeline-empty">
            failed to load: {timeline.error}
          </p>
        ) : timeline.events && timeline.events.length > 0 ? (
          <>
            <div className="drill-in-timeline-controls">
              <button
                type="button"
                className="drill-in-timeline-toggle"
                data-test="drill-in-timeline-toggle"
                aria-pressed={timelineShowAll}
                onClick={() => setTimelineShowAll((v) => !v)}
              >
                {timelineShowAll ? 'Show major gates only' : 'Show all events'}
              </button>
            </div>
            {renderTimelineList(timeline.events, timelineShowAll, story)}
          </>
        ) : (
          <p className="drill-in-timeline-empty">no events yet</p>
        )}
      </details>

      {canAct ? (
        <footer className="drill-in-actions">
          {mode === 'idle' ? (
            <menu className="drill-in-action-row" data-test="drill-in-actions">
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--primary"
                  disabled={busy !== null}
                  onClick={() => {
                    void handleAccept()
                  }}
                  data-test="drill-in-accept"
                >
                  {busy === 'accept' ? 'Accepting…' : 'Accept'}
                </button>
              </li>
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--secondary"
                  disabled={busy !== null}
                  onClick={() => setMode('rejecting')}
                  data-test="drill-in-reject"
                >
                  Reject
                </button>
              </li>
            </menu>
          ) : null}
          <dialog
            ref={rejectDialogRef}
            className="drill-in-reject-form"
            aria-label="Reject story"
            onClose={() => {
              if (mode === 'rejecting') setMode('idle')
            }}
          >
            <form
              onSubmit={(event) => {
                event.preventDefault()
                void handleRejectSend()
              }}
            >
              <label className="drill-in-reject-label">
                <small>Rejection note (required)</small>
                <textarea
                  ref={textareaRef}
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  onKeyDown={handleTextareaKeyDown}
                  placeholder="What needs rework?"
                  rows={3}
                  data-test="drill-in-reject-note"
                />
              </label>
              <menu className="drill-in-action-row">
                <li>
                  <button
                    type="submit"
                    className="drill-in-btn drill-in-btn--primary"
                    disabled={busy !== null || note.trim().length === 0}
                    data-test="drill-in-reject-send"
                  >
                    {busy === 'reject' ? 'Sending…' : 'Send'}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    className="drill-in-btn drill-in-btn--ghost"
                    disabled={busy !== null}
                    onClick={() => setMode('idle')}
                  >
                    Cancel
                  </button>
                </li>
              </menu>
            </form>
          </dialog>
          <dialog
            open={mode === 'bug-question'}
            className="drill-in-bug-question"
            aria-label="Bug-question test"
            data-test="drill-in-bug-question"
          >
            <h3>Would you file a bug about this story if you accepted it as-is?</h3>
            <p>
              The bug-question test catches accept-and-file-bug-as-deferral. If you would file
              a bug, the story still has work — use Reject instead.
            </p>
            <menu className="drill-in-action-row">
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--primary"
                  disabled={busy !== null}
                  onClick={handleAcceptConfirm}
                  data-test="drill-in-bug-question-no"
                >
                  No, accept it
                </button>
              </li>
              <li>
                <button
                  type="button"
                  className="drill-in-btn drill-in-btn--secondary"
                  disabled={busy !== null}
                  onClick={handleAcceptRouteToReject}
                  data-test="drill-in-bug-question-yes"
                >
                  Yes, reject instead
                </button>
              </li>
            </menu>
            <details className="drill-in-bypass">
              <summary>Override bug-question (require justification)</summary>
              <form
                onSubmit={(event) => {
                  event.preventDefault()
                  void handleAcceptBypass()
                }}
              >
                <label className="drill-in-reject-label">
                  <small>Bypass reason (required, recorded in audit trail)</small>
                  <textarea
                    value={bypassReason}
                    onChange={(e) => setBypassReason(e.target.value)}
                    placeholder="Why is this bypass justified?"
                    rows={3}
                    data-test="drill-in-bypass-reason"
                  />
                </label>
                <menu className="drill-in-action-row">
                  <li>
                    <button
                      type="submit"
                      className="drill-in-btn drill-in-btn--secondary"
                      disabled={busy !== null || bypassReason.trim().length === 0}
                      data-test="drill-in-bypass-submit"
                    >
                      Force accept
                    </button>
                  </li>
                </menu>
              </form>
            </details>
          </dialog>
        </footer>
      ) : null}

      {toast ? (
        <aside className="drill-in-toast" role="status" data-test="drill-in-toast">
          {toast}
        </aside>
      ) : null}
    </article>
  )
}

const BackArrow = (): React.JSX.Element => (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      aria-hidden="true"
      focusable="false"
      className="drill-in-back-arrow"
    >
      <path
        d="M7.5 2L3 6l4.5 4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )

export default DrillIn
