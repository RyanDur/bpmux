import { useCallback, useEffect, useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import { useDraftStorage } from './useDraftStorage'
import { useInputHistory } from './useInputHistory'
import './Input.css'

interface Props {
  onSend: (text: string, addressee: string | null) => void
  disabled?: boolean
  placeholder?: string
  channelKey: string
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

const setCaret = (ta: HTMLTextAreaElement, pos: number): void => {
  ta.selectionStart = pos
  ta.selectionEnd = pos
}

const copyToClipboard = (text: string): void => {
  if (!text) return
  if (typeof navigator === 'undefined' || !navigator.clipboard) return
  void navigator.clipboard.writeText(text).catch(() => {
    return
  })
}

interface SearchState {
  query: string
  matchIndex: number | null
  priorValue: string
}

const Input = ({ onSend, disabled, placeholder, channelKey }: Props): React.JSX.Element => {
  const { value, setValue } = useDraftStorage(channelKey)
  const history = useInputHistory()
  const [search, setSearch] = useState<SearchState | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  useEffect(() => {
    if (!disabled) textareaRef.current?.focus()
  }, [disabled])

  const applySearchMatch = useCallback(
    (idx: number | null): void => {
      if (idx === null) return
      const recalled = history.valueAt(idx)
      if (recalled !== undefined) setValue(recalled)
    },
    [history, setValue]
  )

  const exitSearch = useCallback(
    (commit: boolean): void => {
      setSearch((prev) => {
        if (!prev) return prev
        if (!commit) {
          setValue(prev.priorValue)
        } else if (prev.matchIndex !== null) {
          history.commitIndex(prev.matchIndex)
        }
        return null
      })
    },
    [history, setValue]
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>): void => {
      if (e.nativeEvent.isComposing) return

      const ta = e.currentTarget
      const pos = ta.selectionStart
      const selEnd = ta.selectionEnd
      const hasSelection = pos !== selEnd
      const { ctrlKey, altKey, metaKey, shiftKey, key } = e

      const applyEdit = (next: string, caret: number): void => {
        flushSync(() => setValue(next))
        setCaret(ta, caret)
      }

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
          const nextIdx =
            search.matchIndex === null
              ? null
              : history.findMatch(search.query, search.matchIndex - 1)
          setSearch({ ...search, matchIndex: nextIdx ?? search.matchIndex })
          if (nextIdx !== null) applySearchMatch(nextIdx)
          return
        }
        if (key === 'Backspace') {
          e.preventDefault()
          const q = search.query.slice(0, -1)
          const idx = q ? history.findMatch(q) : null
          setSearch({ ...search, query: q, matchIndex: idx })
          if (idx !== null) applySearchMatch(idx)
          else setValue(search.priorValue)
          return
        }
        if (!ctrlKey && !altKey && !metaKey && key.length === 1) {
          e.preventDefault()
          const q = search.query + key
          const idx = history.findMatch(q)
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
        history.record(text)
        setValue('')
        return
      }

      const recallPrev = (): void => {
        const recalled = history.prev(value)
        if (recalled === null) return
        setValue(recalled)
        setCaret(ta, recalled.length)
      }

      const recallNext = (): void => {
        const recalled = history.next()
        if (recalled === null) return
        setValue(recalled)
        setCaret(ta, recalled.length)
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
          recallPrev()
          return
        }
      }

      if (key === 'ArrowDown' && !ctrlKey && !altKey && !metaKey && !shiftKey) {
        const lastLine = !value.slice(pos).includes('\n')
        if (lastLine) {
          e.preventDefault()
          recallNext()
          return
        }
      }

      if (ctrlKey && !altKey && !metaKey) {
        switch (key.toLowerCase()) {
          case 'p': {
            e.preventDefault()
            recallPrev()
            return
          }
          case 'n': {
            e.preventDefault()
            recallNext()
            return
          }
          case 'a': {
            e.preventDefault()
            setCaret(ta, lineStart(value, pos))
            return
          }
          case 'e': {
            e.preventDefault()
            setCaret(ta, lineEnd(value, pos))
            return
          }
          case 'b': {
            e.preventDefault()
            setCaret(ta, Math.max(0, pos - 1))
            return
          }
          case 'f': {
            e.preventDefault()
            setCaret(ta, Math.min(value.length, pos + 1))
            return
          }
          case 'd': {
            e.preventDefault()
            if (hasSelection) {
              applyEdit(value.slice(0, pos) + value.slice(selEnd), pos)
            } else if (pos < value.length) {
              applyEdit(value.slice(0, pos) + value.slice(pos + 1), pos)
            }
            return
          }
          case 'k': {
            e.preventDefault()
            const end = lineEnd(value, pos)
            const killEnd = end === pos ? end + 1 : end
            copyToClipboard(value.slice(pos, killEnd))
            applyEdit(value.slice(0, pos) + value.slice(killEnd), pos)
            return
          }
          case 'u': {
            e.preventDefault()
            const start = lineStart(value, pos)
            copyToClipboard(value.slice(start, pos))
            applyEdit(value.slice(0, start) + value.slice(pos), start)
            return
          }
          case 'w': {
            e.preventDefault()
            const wordStart = prevWordBoundary(value, pos)
            copyToClipboard(value.slice(wordStart, pos))
            applyEdit(value.slice(0, wordStart) + value.slice(pos), wordStart)
            return
          }
          default:
            break
        }
      }

      if (altKey && !ctrlKey && !metaKey) {
        switch (key.toLowerCase()) {
          case 'b': {
            e.preventDefault()
            setCaret(ta, prevWordBoundary(value, pos))
            return
          }
          case 'f': {
            e.preventDefault()
            setCaret(ta, nextWordBoundary(value, pos))
            return
          }
          case 'd': {
            e.preventDefault()
            const wordEnd = nextWordBoundary(value, pos)
            copyToClipboard(value.slice(pos, wordEnd))
            applyEdit(value.slice(0, pos) + value.slice(wordEnd), pos)
            return
          }
          default:
            break
        }
      }
    },
    [value, onSend, search, history, applySearchMatch, exitSearch, setValue]
  )

  return (
    <form className="chat-input" data-test="chat-input" onSubmit={(e) => e.preventDefault()}>
      {search !== null && (
        <output
          className={`chat-input-search${search.matchIndex === null && search.query ? ' chat-input-search--nomatch' : ''}`}
          aria-live="polite"
          data-test="chat-input-isearch"
        >
          <small>
            {search.matchIndex === null && search.query ? (
              <>
                <i className="chat-input-search-fail-glyph" aria-hidden="true">
                  ✗
                </i>
                {` failing i-search: ${search.query}`}
              </>
            ) : (
              `i-search: ${search.query}`
            )}
          </small>
        </output>
      )}
      <textarea
        key={disabled ? 'disabled' : 'enabled'}
        ref={textareaRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder ?? 'type here — Enter to send, Shift+Enter for newline'}
        rows={1}
        aria-label="chat message"
        data-test="chat-input-textarea"
        disabled={disabled}
      />
    </form>
  )
}

export default Input
