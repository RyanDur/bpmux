import { useCallback, useEffect, useRef, useState } from 'react'
import './Input.css'

interface Props {
  onSend: (text: string, addressee?: string | null) => void
  disabled?: boolean
}

const VALID_ADDRESSEES = new Set(['ryan', 'dev', 'pm', 'designer'])

function parseAddressee(text: string): string | null {
  const match = text.match(/^@(\w+)\b/)
  if (!match) return null
  const role = match[1].toLowerCase()
  return VALID_ADDRESSEES.has(role) ? role : null
}

function isWordChar(ch: string | undefined): boolean {
  if (!ch) return false
  return /[\w]/.test(ch)
}

function prevWordBoundary(value: string, from: number): number {
  let i = from
  while (i > 0 && !isWordChar(value[i - 1])) i -= 1
  while (i > 0 && isWordChar(value[i - 1])) i -= 1
  return i
}

function nextWordBoundary(value: string, from: number): number {
  let i = from
  while (i < value.length && !isWordChar(value[i])) i += 1
  while (i < value.length && isWordChar(value[i])) i += 1
  return i
}

function lineStart(value: string, from: number): number {
  const nl = value.lastIndexOf('\n', from - 1)
  return nl === -1 ? 0 : nl + 1
}

function lineEnd(value: string, from: number): number {
  const nl = value.indexOf('\n', from)
  return nl === -1 ? value.length : nl
}

function Input({ onSend, disabled }: Props): React.JSX.Element {
  const [value, setValue] = useState('')
  const taRef = useRef<HTMLTextAreaElement>(null)
  const yankRef = useRef<string>('')

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

  const applyEdit = useCallback(
    (next: string, caret: number, yank?: string): void => {
      setValue(next)
      if (typeof yank === 'string') yankRef.current = yank
      requestAnimationFrame(() => {
        const ta = taRef.current
        if (!ta) return
        ta.selectionStart = caret
        ta.selectionEnd = caret
      })
    },
    []
  )

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

      if (key === 'Enter' && !shiftKey) {
        e.preventDefault()
        const text = value.trim()
        if (!text) return
        onSend(text, parseAddressee(text))
        setValue('')
        return
      }

      if (metaKey && !ctrlKey && !altKey && key.toLowerCase() === 'k') {
        e.preventDefault()
        applyEdit('', 0)
        return
      }

      if (ctrlKey && !altKey && !metaKey) {
        switch (key.toLowerCase()) {
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
            applyEdit(next, pos, killed)
            return
          }
          case 'u': {
            e.preventDefault()
            const start = lineStart(value, pos)
            const killed = value.slice(start, pos)
            const next = value.slice(0, start) + value.slice(pos)
            applyEdit(next, start, killed)
            return
          }
          case 'w': {
            e.preventDefault()
            const wordStart = prevWordBoundary(value, pos)
            const killed = value.slice(wordStart, pos)
            const next = value.slice(0, wordStart) + value.slice(pos)
            applyEdit(next, wordStart, killed)
            return
          }
          case 'y': {
            e.preventDefault()
            const yanked = yankRef.current
            if (!yanked) return
            const next = value.slice(0, pos) + yanked + value.slice(selEnd)
            applyEdit(next, pos + yanked.length)
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
            applyEdit(next, pos, killed)
            return
          }
          default:
            break
        }
      }
    },
    [value, onSend, applyEdit, moveCaret]
  )

  return (
    <form
      className="chat-input"
      data-test="chat-input"
      onSubmit={(e) => e.preventDefault()}
    >
      <textarea
        ref={taRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="type here — Enter to send, Shift+Enter for newline"
        rows={1}
        aria-label="chat message"
        data-test="chat-input-textarea"
      />
    </form>
  )
}

export default Input
