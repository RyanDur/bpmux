import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import MessageBody from '../chat/MessageBody'
import { useAnimatedUnmount } from '../hooks/useAnimatedUnmount'
import { BOARD_INVALIDATE_MARKER, type ChatMessage } from '../transport/types'
import StoryTypeGlyph from './StoryTypeGlyph'
import { storyTypeFromLabels, type BoardStory } from './types'
import './DrillIn.css'

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
  // When set, drill-in scrolls the comments list to this id after the
  // fetch resolves and briefly highlights the matched row. Drives
  // the "click comment-search-result → see the comment" path from
  // #344.
  scrollToCommentId?: number | null
  // #354 close-direction: Board's useAnimatedUnmount flips this to
  // false to trigger the exit transition before unmount.
  visible: boolean
  onBack: () => void
  onStoryMutated: (story: BoardStory) => void
}

function isInvalidateMarker(m: ChatMessage): boolean {
  return m.text === BOARD_INVALIDATE_MARKER
}

// Merge the server-authoritative list with any lingering optimistic
// comments. We trust the server for its ids; an optimistic comment
// whose body matches a server comment (by the same author, within a
// 10-second window) is assumed replaced by the server version and
// dropped. Anything else optimistic stays — covers the "POST pending,
// refetch fired anyway" window.
function mergeComments(
  server: readonly Comment[],
  optimistic: readonly Comment[]
): Comment[] {
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

type ActionMode = 'idle' | 'rejecting'
type BusyKind = 'accept' | 'reject' | null

const TOAST_MS = 4000

function DrillIn({
  story,
  isOwner,
  apiBase,
  authToken,
  usingMock,
  messages,
  scrollToCommentId,
  visible,
  onBack,
  onStoryMutated
}: Props): React.JSX.Element {
  const [mode, setMode] = useState<ActionMode>('idle')
  const [busy, setBusy] = useState<BusyKind>(null)
  const [note, setNote] = useState<string>('')
  const [toast, setToast] = useState<string | null>(null)
  const [serverComments, setServerComments] = useState<Comment[] | null>(null)
  const [optimistic, setOptimistic] = useState<Comment[]>([])
  const [commentDraft, setCommentDraft] = useState<string>('')
  const [commentBusy, setCommentBusy] = useState<boolean>(false)
  const articleRef = useRef<HTMLElement | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
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

  // Reject-form close animation (#358). Keep the form mounted for
  // 150ms after the user dismisses so the grid-row height +
  // opacity can animate out. CSS-side switches to [data-visible]
  // for enter vs exit; default state = closed/exiting, visible
  // = entered.
  const rejectForm = useAnimatedUnmount(mode === 'rejecting', 150)

  // Focus the article on mount so Esc falls to our handler before
  // anything upstream intercepts.
  useEffect(() => {
    articleRef.current?.focus()
  }, [])

  // Autofocus the reject textarea when the reject flow opens so the
  // owner can start typing the note immediately.
  useEffect(() => {
    if (mode === 'rejecting') {
      textareaRef.current?.focus()
    }
  }, [mode])

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

  // Search-result navigation (#344 AC4): when DrillIn opens with a
  // scrollToCommentId, find the matching row after the fetch lands
  // and scroll it into view. Runs whenever the target comment id
  // changes or the comments list becomes available. Takes precedence
  // over the at-bottom autoscroll above because the user explicitly
  // asked to jump to a specific hit.
  useEffect(() => {
    if (scrollToCommentId === null || scrollToCommentId === undefined) return
    if (comments === null) return
    const el = commentsListRef.current
    if (!el) return
    const target = el.querySelector<HTMLLIElement>(
      `[data-comment-id="${scrollToCommentId}"]`
    )
    if (target) {
      target.scrollIntoView({ block: 'center', behavior: 'auto' })
      target.classList.add('drill-in-comment--highlight')
      // Highlight fades via CSS transition; class remains on the
      // element so a re-scroll to the same comment re-triggers the
      // CSS animation via React removing/reapplying it won't happen
      // in practice since scrollToCommentId only changes on
      // navigation. Clean up by removing after the animation window.
      const id = setTimeout(() => {
        target.classList.remove('drill-in-comment--highlight')
      }, 2_000)
      return () => clearTimeout(id)
    }
    return
  }, [scrollToCommentId, comments])

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

  const handleAccept = useCallback(async () => {
    if (busy !== null) return
    setBusy('accept')
    const rollback = (reason: string): void => {
      setBusy(null)
      setToast(`Accept failed: ${reason}`)
    }
    try {
      if (usingMock || !apiBase) {
        // Optimistic local mutation in mock mode so the UX is viewable
        // without a backend.
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
        }
      })
      if (!res.ok) {
        rollback(`HTTP ${res.status}`)
        return
      }
      onStoryMutated({ ...story, state: 'accepted' })
      setBusy(null)
      onBack()
    } catch (err) {
      rollback(err instanceof Error ? err.message : String(err))
    }
  }, [apiBase, authToken, busy, onBack, onStoryMutated, story, usingMock])

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
        onStoryMutated({ ...story, state: 'in-progress' })
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
        rollback(`HTTP ${res.status}`)
        return
      }
      onStoryMutated({ ...story, state: 'in-progress' })
      setBusy(null)
      setMode('idle')
      setNote('')
      onBack()
    } catch (err) {
      rollback(err instanceof Error ? err.message : String(err))
    }
  }, [apiBase, authToken, busy, note, onBack, onStoryMutated, story, usingMock])

  // #366 quote-excerpt: Cmd/Ctrl+E with a text selection inside any
  // rendered comment body prepends a markdown blockquote of the
  // selection to the comment draft, then focuses the textarea so the
  // author can continue composing. The anchor in the blockquote links
  // back to the source comment via `#comment-<id>`, rendered by
  // MessageBody as a clickable scroll-to-origin link. Rationale:
  // concurrent-write pain — Ryan observed that mid-compose, new
  // comments arrive and his reply's antecedent becomes ambiguous. An
  // explicit quote-of-text + author marker removes the ambiguity
  // without requiring full threaded-reply substrate.
  const handleQuoteShortcut = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      if (event.key !== 'e' && event.key !== 'E') return
      if (!(event.metaKey || event.ctrlKey)) return
      if (event.altKey || event.shiftKey) return
      const sel = window.getSelection()
      if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return
      const anchor = sel.anchorNode
      if (!anchor) return
      // Walk up from the selection anchor until we find the comment
      // <li> (bearing data-comment-id). If the selection started
      // outside a comment body, we don't quote.
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
      const quoted = selected
        .split('\n')
        .map((line) => `> ${line}`)
        .join('\n')
      const header = `> [@${author}](#comment-${id}):`
      const prefix = `${header}\n${quoted}\n\n`
      setCommentDraft((prev) => prefix + prev)
      // Defer focus + cursor placement to the next frame so React has
      // applied the state update and the textarea's value reflects the
      // new prefix. Cursor lands at prefix.length so the user types
      // AFTER the blank line that follows the quote block, matching
      // the AC's "<cursor here>" placement between quote + existing
      // draft content.
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

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLElement>) => {
      // Quote-shortcut runs first — if it consumes the event
      // (selection inside a comment + Cmd/Ctrl+E), we stop here.
      handleQuoteShortcut(event)
      if (event.defaultPrevented) return
      if (event.key === 'Escape') {
        event.preventDefault()
        if (mode === 'rejecting') {
          setMode('idle')
          // preserve note text per spec
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
        <dl className="drill-in-meta-list">
          <dt>state</dt>
          <dd>{story.state}</dd>
          <dt>labels</dt>
          <dd>
            {story.labels.length === 0 ? (
              <em>(none)</em>
            ) : (
              <ul className="drill-in-chips">
                {story.labels.map((label) => (
                  <li key={label} className="drill-in-chip">
                    {label}
                  </li>
                ))}
              </ul>
            )}
          </dd>
          <dt>assignees</dt>
          <dd>
            {story.assignees.length === 0 ? (
              <em>(none)</em>
            ) : (
              <ul className="drill-in-chips">
                {story.assignees.map((a) => (
                  <li key={a} className="drill-in-chip">
                    {a}
                  </li>
                ))}
              </ul>
            )}
          </dd>
        </dl>
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

      {canAct ? (
        <footer className="drill-in-actions">
          {mode === 'idle' && !rejectForm.mounted ? (
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
          ) : rejectForm.mounted ? (
            <form
              className="drill-in-reject-form"
              data-visible={rejectForm.visible || undefined}
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
          ) : null}
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

function BackArrow(): React.JSX.Element {
  return (
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
}

export default DrillIn
