import { useEffect, useRef, useState } from 'react'
import type { AggregatedReaction } from '../hooks/useReactions'
import './Reactions.css'

export const EMOJI_PALETTE = ['👍', '🎉', '❤️', '🚀', '👀', '❌'] as const

const PICKER_HEIGHT_ESTIMATE = 44

type PickerPosition = 'above' | 'below'

interface PickerProps {
  anchorRef: React.RefObject<HTMLButtonElement | null>
  onPick: (emoji: string) => void
  onClose: () => void
}

const Picker = ({ anchorRef, onPick, onClose }: PickerProps): React.JSX.Element => {
  const pickerRef = useRef<HTMLUListElement | null>(null)
  const [position, setPosition] = useState<PickerPosition>('above')

  useEffect(() => {
    const anchor = anchorRef.current
    if (!anchor) return
    const rect = anchor.getBoundingClientRect()
    const needed = PICKER_HEIGHT_ESTIMATE + 8
    const hasSpaceAbove = rect.top >= needed
    setPosition(hasSpaceAbove ? 'above' : 'below')
  }, [anchorRef])

  useEffect(() => {
    const handleKey = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onClose()
      }
    }
    const handleClick = (e: MouseEvent): void => {
      const picker = pickerRef.current
      const anchor = anchorRef.current
      const target = e.target as Node | null
      if (!target) return
      if (picker?.contains(target)) return
      if (anchor?.contains(target)) return
      onClose()
    }
    document.addEventListener('keydown', handleKey, true)
    document.addEventListener('mousedown', handleClick, true)
    return () => {
      document.removeEventListener('keydown', handleKey, true)
      document.removeEventListener('mousedown', handleClick, true)
    }
  }, [anchorRef, onClose])

  return (
    <ul
      ref={pickerRef}
      className={`reaction-picker reaction-picker--${position}`}
      data-test="reaction-picker"
      data-position={position}
      role="menu"
      aria-label="Pick a reaction"
    >
      {EMOJI_PALETTE.map((e) => (
        <li key={e} role="none">
          <button
            type="button"
            className="reaction-picker-item"
            data-test={`reaction-picker-${e}`}
            aria-label={`React with ${e}`}
            role="menuitem"
            onClick={() => {
              onPick(e)
            }}
          >
            {e}
          </button>
        </li>
      ))}
    </ul>
  )
}

interface Props {
  reactions: AggregatedReaction[] | undefined
  viewer: string
  onToggle: (emoji: string) => void
}

const Reactions = ({ reactions, viewer, onToggle }: Props): React.JSX.Element | null => {
  const [pickerOpen, setPickerOpen] = useState<boolean>(false)
  const addButtonRef = useRef<HTMLButtonElement | null>(null)
  const pills = reactions ?? []

  if (pills.length === 0 && !pickerOpen) {
    return null
  }

  return (
    <footer className="message-reactions" data-test="message-reactions">
      {pills.map((r) => {
        const reacted = r.users.includes(viewer)
        return (
          <button
            key={r.emoji}
            type="button"
            className={`reaction-pill${reacted ? ' reaction-pill--reacted' : ''}`}
            data-test={`reaction-pill-${r.emoji}`}
            aria-label={`${r.emoji} ${r.count} reactions from ${r.users.join(', ')}`}
            title={r.users.join(', ')}
            aria-pressed={reacted}
            onClick={() => onToggle(r.emoji)}
          >
            <i aria-hidden="true">{r.emoji}</i>
            <small>{r.count}</small>
          </button>
        )
      })}
      <aside className="reaction-add-anchor">
        <button
          ref={addButtonRef}
          type="button"
          className="reaction-add"
          data-test="reaction-add"
          aria-label={pickerOpen ? 'Close reaction picker' : 'Add reaction'}
          aria-haspopup="menu"
          aria-expanded={pickerOpen}
          title={pickerOpen ? 'Close' : 'Add reaction'}
          onClick={() => setPickerOpen((prev) => !prev)}
        >
          {pickerOpen ? '✕' : '+😊'}
        </button>
        {pickerOpen && (
          <Picker
            anchorRef={addButtonRef}
            onPick={(e) => {
              onToggle(e)
              setPickerOpen(false)
            }}
            onClose={() => setPickerOpen(false)}
          />
        )}
      </aside>
    </footer>
  )
}

interface AddOnlyProps {
  onToggle: (emoji: string) => void
}

export const ReactionAddButton = ({ onToggle }: AddOnlyProps): React.JSX.Element => {
  const [open, setOpen] = useState<boolean>(false)
  const buttonRef = useRef<HTMLButtonElement | null>(null)
  return (
    <aside className="reaction-add-container">
      <button
        ref={buttonRef}
        type="button"
        className="reaction-add"
        data-test="reaction-add-first"
        aria-label={open ? 'Close reaction picker' : 'Add reaction'}
        aria-haspopup="menu"
        aria-expanded={open}
        title={open ? 'Close' : 'Add reaction'}
        onClick={() => setOpen((prev) => !prev)}
      >
        {open ? '✕' : '+😊'}
      </button>
      {open && (
        <Picker
          anchorRef={buttonRef}
          onPick={(e) => {
            onToggle(e)
            setOpen(false)
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </aside>
  )
}

export default Reactions
