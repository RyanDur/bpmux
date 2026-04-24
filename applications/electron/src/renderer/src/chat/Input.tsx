import { useCallback, useEffect, useRef, useState } from 'react'
import './Input.css'

interface Props {
  onSend: (text: string, addressee: string | null) => void
  disabled?: boolean
  placeholder?: string
  channelKey: string
}

const DRAFT_STORAGE_PREFIX = 'bpmux-draft:'

const readDraft = (channelKey: string): string => {
  if (typeof window === 'undefined') return ''
  try {
    return window.localStorage.getItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`) ?? ''
  } catch {
    return ''
  }
}

const writeDraft = (channelKey: string, value: string): void => {
  if (typeof window === 'undefined') return
  try {
    if (value.length === 0) {
      window.localStorage.removeItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`)
    } else {
      window.localStorage.setItem(`${DRAFT_STORAGE_PREFIX}${channelKey}`, value)
    }
  } catch {
    return
  }
}

const VALID_ADDRESSEES = new Set(['ryan', 'dev', 'pm', 'designer'])

const parseAddressee = (text: string): string | null => {
  const match = text.match(/^@(\w+)\b/)
  if (!match) return null
  const role = match[1].toLowerCase()
  return VALID_ADDRESSEES.has(role) ? role : null
}

const isWordChar = (ch: string | undefined): boolean => {
  if (!ch) return false
  return /[\w]/.test(ch)
}

const prevWordBoundary = (value: string, from: number): number => {
  let i = from
  while (i > 0 && !isWordChar(value[i - 1])) i -= 1
  while (i > 0 && isWordChar(value[i - 1])) i -= 1
  return i
}

const nextWordBoundary = (value: string, from: number): number => {
  let i = from
  while (i < value.length && !isWordChar(value[i])) i += 1
  while (i < value.length && isWordChar(value[i])) i += 1
  return i
}

const lineStart = (value: string, from: number): number => {
  const nl = value.lastIndexOf('\n', from - 1)
  return nl === -1 ? 0 : nl + 1
}

const lineEnd = (value: string, from: number): number => {
  const nl = value.indexOf('\n', from)
  return nl === -1 ? value.length : nl
}

const MAX_HISTORY = 50

interface SearchState {
  query: string
  matchIndex: number | null
  priorValue: string
}

const Input = ({ onSend, disabled, placeholder, channelKey }: Props): React.JSX.Element => {
  const [value, setValue] = useState(() => readDraft(channelKey))
  const [search, setSearch] = useState<SearchState | null>(null)
  const channelKeyRef = useRef(channelKey)

  useEffect(() => {
    if (channelKeyRef.current === channelKey) return
    writeDraft(channelKeyRef.current, value)
    channelKeyRef.current = channelKey
    setValue(readDraft(channelKey))
  }, [channelKey, value])

  useEffect(() => {
    writeDraft(channelKey, value)
  }, [channelKey, value])
  const taRef = useRef<HTMLTextAreaElement>(null)
  const historyRef = useRef<string[]>([])
  const historyIndexRef = useRef<number | null>(null)
  const draftRef = useRef<string>('')

  // #378: kill-ops (Ctrl+W/K/U + Alt+D) pipe killed text to the
  // system clipboard so users can restore via the platform-native
  // Cmd+V. Retires the internal `yankRef` + `Ctrl+Y` binding —
  // emacs-strict yank didn't match typical Mac/Electron muscle
  // memory, and routing to the clipboard gives one consistent
  // "paste" story (OS clipboard for everything). Fire-and-forget
  // because failures aren't user-actionable and the event loop
  // must keep going; user-initiated event satisfies Clipboard-API
  // permission semantics.
  const copyToClipboard = useCallback((text: string): void => {
    if (!text) return
    if (typeof navigator === 'undefined' || !navigator.clipboard) return
    void navigator.clipboard.writeText(text).catch(() => {
      // Permission denied / navigator.clipboard unavailable under
      // the test harness. The kill-op still succeeds (text was
      // removed from the textarea); clipboard-write is a best-
      // effort upgrade from the old yank-into-oblivion path.
    })
  }, [])

  const findMatch = useCallback(
    (query: string, startFrom: number): number | null => {
      if (!query) return null
      const history = historyRef.current
      for (let i = startFrom; i >= 0; i -= 1) {
        if (history[i].includes(query)) return i
      }
      return null
    },
    []
  )

  const applySearchMatch = useCallback((idx: number | null): void => {
    if (idx === null) return
    const history = historyRef.current
    setValue(history[idx])
  }, [])

  const exitSearch = useCallback((commit: boolean): void => {
    setSearch((prev) => {
      if (!prev) return prev
      if (!commit) {
        setValue(prev.priorValue)
      } else if (prev.matchIndex !== null) {
        historyIndexRef.current = prev.matchIndex
      }
      return null
    })
  }, [])

  useEffect(() => {
    if (!disabled) taRef.current?.focus()
  }, [disabled])

  useEffect(() => {
    const ta = taRef.current
    if (!ta) return
    ta.style.height = 'auto'
    const maxHeight = 144
    ta.style.height = `${Math.min(ta.scrollHeight, maxHeight)}px`
  }, [value])

  const applyEdit = useCallback((next: string, caret: number): void => {
    setValue(next)
    requestAnimationFrame(() => {
      const ta = taRef.current
      if (!ta) return
      ta.selectionStart = caret
      ta.selectionEnd = caret
    })
  }, [])

  const moveCaret = useCallback((pos: number): void => {
    requestAnimationFrame(() => {
      const ta = taRef.current
      if (!ta) return
      ta.selectionStart = pos
      ta.selectionEnd = pos
    })
  }, [])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>): void => {
      if (e.nativeEvent.isComposing) return

      const ta = e.currentTarget
      const pos = ta.selectionStart
      const selEnd = ta.selectionEnd
      const hasSelection = pos !== selEnd
      const { ctrlKey, altKey, metaKey, shiftKey, key } = e

      if (search !== null) {
        if (key === 'Escape') {
          e.preventDefault()
          exitSearch(false)
          return
        }
        if (key === 'Enter') {
          e.preventDefault()
          exitSearch(true)
          return
        }
        if (ctrlKey && !altKey && !metaKey && key.toLowerCase() === 'r') {
          e.preventDefault()
          const nextIdx = search.matchIndex === null ? null : findMatch(search.query, search.matchIndex - 1)
          setSearch({ ...search, matchIndex: nextIdx ?? search.matchIndex })
          if (nextIdx !== null) applySearchMatch(nextIdx)
          return
        }
        if (key === 'Backspace') {
          e.preventDefault()
          const q = search.query.slice(0, -1)
          const history = historyRef.current
          const idx = q ? findMatch(q, history.length - 1) : null
          setSearch({ ...search, query: q, matchIndex: idx })
          if (idx !== null) applySearchMatch(idx)
          else setValue(search.priorValue)
          return
        }
        if (!ctrlKey && !altKey && !metaKey && key.length === 1) {
          e.preventDefault()
          const q = search.query + key
          const history = historyRef.current
          const idx = findMatch(q, history.length - 1)
          setSearch({ ...search, query: q, matchIndex: idx })
          if (idx !== null) applySearchMatch(idx)
          return
        }
      }

      if (ctrlKey && !altKey && !metaKey && key.toLowerCase() === 'r') {
        e.preventDefault()
        setSearch({ query: '', matchIndex: null, priorValue: value })
        return
      }


      if (key === 'Enter' && !shiftKey) {
        e.preventDefault()
        const text = value.trim()
        if (!text) return
        onSend(text, parseAddressee(text))
        const history = historyRef.current
        if (history[history.length - 1] !== text) {
          history.push(text)
          if (history.length > MAX_HISTORY) history.shift()
        }
        historyIndexRef.current = null
        draftRef.current = ''
        setValue('')
        return
      }

      const historyPrev = (): void => {
        const history = historyRef.current
        if (history.length === 0) return
        const idx = historyIndexRef.current
        if (idx === null) {
          draftRef.current = value
          historyIndexRef.current = history.length - 1
        } else if (idx > 0) {
          historyIndexRef.current = idx - 1
        } else {
          return
        }
        const recalled = history[historyIndexRef.current]
        setValue(recalled)
        moveCaret(recalled.length)
      }

      const historyNext = (): void => {
        const idx = historyIndexRef.current
        if (idx === null) return
        const history = historyRef.current
        if (idx < history.length - 1) {
          historyIndexRef.current = idx + 1
          const recalled = history[historyIndexRef.current]
          setValue(recalled)
          moveCaret(recalled.length)
        } else {
          historyIndexRef.current = null
          const draft = draftRef.current
          setValue(draft)
          moveCaret(draft.length)
        }
      }

      if (metaKey && !ctrlKey && !altKey && key.toLowerCase() === 'k') {
        e.preventDefault()
        applyEdit('', 0)
        return
      }

      if (key === 'ArrowUp' && !ctrlKey && !altKey && !metaKey && !shiftKey) {
        const firstLine = !value.slice(0, pos).includes('\n')
        if (firstLine) {
          e.preventDefault()
          historyPrev()
          return
        }
      }

      if (key === 'ArrowDown' && !ctrlKey && !altKey && !metaKey && !shiftKey) {
        const lastLine = !value.slice(pos).includes('\n')
        if (lastLine) {
          e.preventDefault()
          historyNext()
          return
        }
      }

      if (ctrlKey && !altKey && !metaKey) {
        switch (key.toLowerCase()) {
          case 'p': {
            e.preventDefault()
            historyPrev()
            return
          }
          case 'n': {
            e.preventDefault()
            historyNext()
            return
          }
          case 'a': {
            e.preventDefault()
            moveCaret(lineStart(value, pos))
            return
          }
          case 'e': {
            e.preventDefault()
            moveCaret(lineEnd(value, pos))
            return
          }
          case 'b': {
            e.preventDefault()
            moveCaret(Math.max(0, pos - 1))
            return
          }
          case 'f': {
            e.preventDefault()
            moveCaret(Math.min(value.length, pos + 1))
            return
          }
          case 'd': {
            e.preventDefault()
            if (hasSelection) {
              const next = value.slice(0, pos) + value.slice(selEnd)
              applyEdit(next, pos)
            } else if (pos < value.length) {
              const next = value.slice(0, pos) + value.slice(pos + 1)
              applyEdit(next, pos)
            }
            return
          }
          case 'k': {
            e.preventDefault()
            const end = lineEnd(value, pos)
            const killed = value.slice(pos, end === pos ? end + 1 : end)
            const next = value.slice(0, pos) + value.slice(end === pos ? end + 1 : end)
            copyToClipboard(killed)
            applyEdit(next, pos)
            return
          }
          case 'u': {
            e.preventDefault()
            const start = lineStart(value, pos)
            const killed = value.slice(start, pos)
            const next = value.slice(0, start) + value.slice(pos)
            copyToClipboard(killed)
            applyEdit(next, start)
            return
          }
          case 'w': {
            e.preventDefault()
            const wordStart = prevWordBoundary(value, pos)
            const killed = value.slice(wordStart, pos)
            const next = value.slice(0, wordStart) + value.slice(pos)
            copyToClipboard(killed)
            applyEdit(next, wordStart)
            return
          }
          // #378: Ctrl+Y yank retired. Users reach for Cmd+V for
          // paste (standard OS clipboard), which is also where our
          // Ctrl+W/K/U kills now land via copyToClipboard above.
          // One-clipboard story, no internal yank-ref.
          default:
            break
        }
      }

      if (altKey && !ctrlKey && !metaKey) {
        switch (key.toLowerCase()) {
          case 'b': {
            e.preventDefault()
            moveCaret(prevWordBoundary(value, pos))
            return
          }
          case 'f': {
            e.preventDefault()
            moveCaret(nextWordBoundary(value, pos))
            return
          }
          case 'd': {
            e.preventDefault()
            const wordEnd = nextWordBoundary(value, pos)
            const killed = value.slice(pos, wordEnd)
            const next = value.slice(0, pos) + value.slice(wordEnd)
            copyToClipboard(killed)
            applyEdit(next, pos)
            return
          }
          default:
            break
        }
      }
    },
    [value, onSend, applyEdit, copyToClipboard, moveCaret, search, findMatch, applySearchMatch, exitSearch]
  )

  return (
    <form
      className="chat-input"
      data-test="chat-input"
      onSubmit={(e) => e.preventDefault()}
    >
      {search !== null && (
        <aside
          className={`chat-input-search${search.matchIndex === null && search.query ? ' chat-input-search--nomatch' : ''}`}
          role="status"
          aria-live="polite"
          data-test="chat-input-isearch"
        >
          <small>
            {search.matchIndex === null && search.query ? (
              <>
                {/* Shape-not-color fallback: the ✗ glyph signals the
                 * failing state even when the nomatch-color cue is
                 * imperceptible (colorblind users, low-contrast
                 * displays). aria-hidden keeps SR reading the
                 * surrounding text naturally ("failing i-search:
                 * foo") without announcing "multiplication sign". */}
                <span className="chat-input-search-fail-glyph" aria-hidden="true">
                  ✗
                </span>{' '}
                failing i-search: {search.query}
              </>
            ) : (
              `i-search: ${search.query}`
            )}
          </small>
        </aside>
      )}
      <textarea
        ref={taRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder ?? 'type here — Enter to send, Shift+Enter for newline'}
        rows={1}
        aria-label="chat message"
        data-test="chat-input-textarea"
      />
    </form>
  )
}

export default Input
