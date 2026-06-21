import { useCallback, useState } from 'react'
import { flushSync } from 'react-dom'
import type { useInputHistory } from './useInputHistory'
import {
  copyToClipboard,
  lineEnd,
  lineStart,
  nextWordBoundary,
  parseAddressee,
  prevWordBoundary,
  setCaret
} from './inputCommands'

export interface SearchState {
  query: string
  matchIndex: number | null
  priorValue: string
}

export interface UseInputKeyboardResult {
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void
  search: SearchState | null
}

export interface SubmitOptions {
  trimOnSubmit?: boolean
  submitEmpty?: boolean
}

export const useInputKeyboard = (
  value: string,
  setValue: (next: string) => void,
  history: ReturnType<typeof useInputHistory>,
  onSend: (text: string, addressee: string | null) => void,
  submit: SubmitOptions = {}
): UseInputKeyboardResult => {
  const { trimOnSubmit = true, submitEmpty = false } = submit
  const [search, setSearch] = useState<SearchState | null>(null)

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
        const text = trimOnSubmit ? value.trim() : value
        if (!text && !submitEmpty) return
        onSend(text, parseAddressee(text))
        if (text) history.record(text)
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
    [
      value,
      onSend,
      search,
      history,
      applySearchMatch,
      exitSearch,
      setValue,
      trimOnSubmit,
      submitEmpty
    ]
  )

  return { handleKeyDown, search }
}
