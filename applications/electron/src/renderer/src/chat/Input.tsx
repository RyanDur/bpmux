import { useEffect, useRef } from 'react'
import { useDraftStorage } from './useDraftStorage'
import { useInputHistory } from './useInputHistory'
import { useInputKeyboard } from './useInputKeyboard'
import './Input.css'

interface Props {
  onSend: (text: string, addressee: string | null) => void
  disabled?: boolean
  placeholder?: string
  channelKey: string
}

const Input = ({ onSend, disabled, placeholder, channelKey }: Props): React.JSX.Element => {
  const { value, setValue } = useDraftStorage(channelKey)
  const history = useInputHistory()
  const { handleKeyDown, search } = useInputKeyboard(value, setValue, history, onSend)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  useEffect(() => {
    if (!disabled) textareaRef.current?.focus()
  }, [disabled])

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
