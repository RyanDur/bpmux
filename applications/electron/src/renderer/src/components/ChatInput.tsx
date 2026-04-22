import { useCallback, useEffect, useRef, useState } from 'react'

interface Props {
  onSend: (text: string) => void
  disabled?: boolean
}

// Phase-1a chat input: textarea with Enter-submits / Shift+Enter-
// newline. Auto-grows as text overflows (up to ~6 lines) before
// scrolling internally. Emacs/readline bindings are phase-1b.
function ChatInput({ onSend, disabled }: Props): React.JSX.Element {
  const [value, setValue] = useState('')
  const taRef = useRef<HTMLTextAreaElement>(null)

  // Auto-focus on mount and re-focus after the parent clears the input
  // (controlled-value to '' triggers this effect). Keeps the
  // "input-always-reachable" invariant frictionless.
  useEffect(() => {
    if (!disabled) taRef.current?.focus()
  }, [disabled])

  // Auto-grow: reset to single line, then expand to scrollHeight so the
  // textarea matches its content without a forever-growing gap.
  useEffect(() => {
    const ta = taRef.current
    if (!ta) return
    ta.style.height = 'auto'
    const maxHeight = 144 // ~6 lines at 1.5rem line-height; overflow scrolls after.
    ta.style.height = `${Math.min(ta.scrollHeight, maxHeight)}px`
  }, [value])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // nativeEvent.isComposing guards against double-submit during
      // IME composition (e.g., Japanese/Chinese input); the React
      // synthetic event doesn't expose this property.
      if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
        e.preventDefault()
        const text = value.trim()
        if (!text) return
        onSend(text)
        setValue('')
      }
    },
    [value, onSend]
  )

  return (
    <div className="chat-input" data-test="chat-input">
      <textarea
        ref={taRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="type here — Enter to send, Shift+Enter for newline"
        rows={1}
        data-test="chat-input-textarea"
      />
    </div>
  )
}

export default ChatInput
