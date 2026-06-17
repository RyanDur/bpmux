import { format } from 'date-fns'
import { useCallback, useEffect, useRef, useState } from 'react'
import MessageBody from '../chat/MessageBody'
import { describeHttpError } from '../api/http'
import { getStoryComments, postStoryComment, type Comment } from '../api/storiesClient'
import { useBoardInvalidateTick } from '../transport/session'

const OPTIMISTIC_ID_BASE = -1_000_000_000

const mergeComments = (server: readonly Comment[], optimistic: readonly Comment[]): Comment[] => {
  const seen = new Set(server.map((c) => `${c.author}|${c.body}`))
  const keepOptimistic = optimistic.filter((o) => !seen.has(`${o.author}|${o.body}`))
  return [...server, ...keepOptimistic].sort((a, b) => a.created_at.localeCompare(b.created_at))
}

interface Props {
  storyNumber: number
  apiBase: string | undefined
  authToken: string | undefined
  usingMock: boolean
  onError: (msg: string) => void
}

const Comments = ({
  storyNumber,
  apiBase,
  authToken,
  usingMock,
  onError
}: Props): React.JSX.Element => {
  const boardInvalidateTick = useBoardInvalidateTick()
  const [fetchedComments, setFetchedComments] = useState<Comment[] | null>(null)
  const [optimistic, setOptimistic] = useState<Comment[]>([])
  const [commentDraft, setCommentDraft] = useState<string>('')
  const [commentBusy, setCommentBusy] = useState<boolean>(false)
  const commentsListRef = useRef<HTMLUListElement | null>(null)
  const commentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const wasAtBottomRef = useRef<boolean>(true)

  const offline = usingMock || apiBase === undefined
  const serverComments: readonly Comment[] | null = offline ? [] : fetchedComments
  const comments = serverComments === null ? null : mergeComments(serverComments, optimistic)

  const captureScroll = useCallback((): void => {
    const el = commentsListRef.current
    if (!el) return
    const max = el.scrollHeight - el.clientHeight
    wasAtBottomRef.current = max <= 0 || el.scrollTop >= max - 4
  }, [])

  const fetchComments = useCallback(
    async (signal?: AbortSignal): Promise<void> => {
      captureScroll()
      if (offline) {
        return
      }
      await getStoryComments({ apiBase, authToken }, storyNumber, signal)
        .onSuccess((c) => setFetchedComments(c))
        .onFailure((error) => {
          if (error.kind === 'network' && error.message.includes('abort')) return
          setFetchedComments([])
        }).value
    },
    [apiBase, authToken, captureScroll, storyNumber, offline]
  )

  useEffect(() => {
    if (offline) return
    captureScroll()
    const controller = new AbortController()
    getStoryComments({ apiBase, authToken }, storyNumber, controller.signal)
      .onSuccess((c) => {
        if (controller.signal.aborted) return
        setFetchedComments(c)
      })
      .onFailure((error) => {
        if (controller.signal.aborted) return
        if (error.kind === 'network' && error.message.includes('abort')) return
        setFetchedComments([])
      })
    return () => controller.abort()
  }, [apiBase, authToken, captureScroll, storyNumber, offline])

  useEffect(() => {
    if (boardInvalidateTick === 0) return
    void fetchComments()
  }, [boardInvalidateTick, fetchComments])

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
    const optimisticEntry: Comment = {
      id: OPTIMISTIC_ID_BASE - Date.now(),
      author: 'ryan',
      body: trimmed,
      created_at: new Date().toISOString()
    }
    setOptimistic((prev) => [...prev, optimisticEntry])
    setCommentDraft('')
    if (usingMock || apiBase === undefined) {
      setCommentBusy(false)
      return
    }
    await postStoryComment({ apiBase, authToken }, storyNumber, { body: trimmed }).onFailure(
      (error) => {
        setOptimistic((prev) => prev.filter((c) => c.id !== optimisticEntry.id))
        setCommentDraft(trimmed)
        onError(`Comment failed: ${describeHttpError(error)}`)
      }
    ).value
    setCommentBusy(false)
  }, [apiBase, authToken, commentBusy, commentDraft, onError, storyNumber, usingMock])

  const insertQuote = useCallback((id: string, author: string, text: string) => {
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
  }, [])

  useEffect(() => {
    const bridge = typeof window !== 'undefined' ? window.bpmux : undefined
    if (!bridge?.onQuoteComment) return
    const unsubscribe = bridge.onQuoteComment(({ commentId, author, text }) => {
      insertQuote(commentId, author, text)
    })
    return unsubscribe
  }, [insertQuote])

  useEffect(() => {
    const handler = (event: KeyboardEvent): void => {
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
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [insertQuote])

  const handleCommentKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleCommentSubmit()
      }
    },
    [handleCommentSubmit]
  )

  return (
    <section className="drill-in-comments" aria-label="Comments">
      <header className="drill-in-comments-header">
        <h4>Comments{comments !== null ? ` (${comments.length})` : ''}</h4>
      </header>
      {comments === null ? (
        <p className="drill-in-comments-empty">loading…</p>
      ) : comments.length === 0 ? (
        <p className="drill-in-comments-empty">no comments yet</p>
      ) : (
        <ul
          className="drill-in-comments-list"
          ref={commentsListRef}
          data-test="drill-in-comments-list"
        >
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
  )
}

export default Comments
