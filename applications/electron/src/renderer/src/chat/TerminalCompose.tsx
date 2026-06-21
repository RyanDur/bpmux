import { useDraftStorage } from './useDraftStorage'
import { useInputHistory } from './useInputHistory'
import { useInputKeyboard } from './useInputKeyboard'
import './TerminalCompose.css'

interface TerminalComposeProps {
  composeKey: string
  onSubmit: (line: string) => void
  inputRef?: React.Ref<HTMLTextAreaElement>
}

const TERMINAL_SUBMIT = { trimOnSubmit: false, submitEmpty: true }

export const TerminalCompose = ({
  composeKey,
  onSubmit,
  inputRef
}: TerminalComposeProps): React.JSX.Element => {
  const { value, setValue } = useDraftStorage(`terminal:${composeKey}`)
  const history = useInputHistory()
  const { handleKeyDown, search } = useInputKeyboard(
    value,
    setValue,
    history,
    (text) => onSubmit(text),
    TERMINAL_SUBMIT
  )

  return (
    <form className="terminal-compose" onSubmit={(e) => e.preventDefault()}>
      {search !== null && (
        <output
          className={`terminal-compose-search${search.matchIndex === null && search.query ? ' terminal-compose-search--nomatch' : ''}`}
          aria-live="polite"
          data-test="terminal-compose-isearch"
        >
          <small>
            {search.matchIndex === null && search.query
              ? `failing i-search: ${search.query}`
              : `i-search: ${search.query}`}
          </small>
        </output>
      )}
      <textarea
        ref={inputRef}
        className="terminal-compose-input"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        rows={1}
        spellCheck={false}
        autoCapitalize="off"
        autoCorrect="off"
        aria-label="terminal input"
        data-test="terminal-compose"
      />
    </form>
  )
}
