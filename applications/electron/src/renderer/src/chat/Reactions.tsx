import { useEffect, useRef, useState } from 'react'
import type { AggregatedReaction } from '../hooks/useReactions'
import './Reactions.css'

// Minimal curated emoji palette (#340). Full Slack-style picker with
// search over the entire Unicode emoji set is out-of-scope for v1;
// these 6 cover the common "ack / +1 / celebrate / ship it" signals
// that reactions are meant to carry in the first place. A richer
// picker can be added as a follow-up when users exhaust the palette.
export const EMOJI_PALETTE = ['👍', '🎉', '❤️', '🚀', '👀', '❌'] as const

// Picker placement heuristic (#355):
//   - Preferred: above the trigger (matches Slack/Discord).
//   - Fallback to below when there isn't enough room above for the
//     picker height + tail.
// Rough picker height estimate — actual measured size drives the
// boundary check; this constant is just the preliminary "would it
// fit" gate while the picker is still invisible.
const PICKER_HEIGHT_ESTIMATE = 44

type PickerPosition = 'above' | 'below'

interface PickerProps {
  anchorRef: React.RefObject<HTMLButtonElement | null>
  onPick: (emoji: string) => void
  onClose: () => void
}

/**
 * Absolute-positioned emoji palette anchored to a trigger button.
 * Computes above/below placement by measuring available space around
 * the anchor's bounding rect on mount, with an automatic flip when
 * the preferred side would clip against the viewport. Esc + click-
 * outside dismiss (#355 AC5).
 */
function Picker({ anchorRef, onPick, onClose }: PickerProps): React.JSX.Element {
  const pickerRef = useRef<HTMLDivElement | null>(null)
  const [position, setPosition] = useState<PickerPosition>('above')

  // Measure anchor position on mount to decide above-vs-below.
  useEffect(() => {
    const anchor = anchorRef.current
    if (!anchor) return
    const rect = anchor.getBoundingClientRect()
    // Estimate needed height; actual could be larger if the picker
    // wraps, but the palette is a single row so this is close
    // enough for the flip decision.
    const needed = PICKER_HEIGHT_ESTIMATE + 8
    const hasSpaceAbove = rect.top >= needed
    setPosition(hasSpaceAbove ? 'above' : 'below')
  }, [anchorRef])

  // Esc + click-outside close (AC5). Listeners on document so they
  // fire regardless of focus position inside the picker.
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
    <div
      ref={pickerRef}
      className={`reaction-picker reaction-picker--${position}`}
      data-test="reaction-picker"
      data-position={position}
      role="menu"
      aria-label="Pick a reaction"
    >
      {EMOJI_PALETTE.map((e) => (
        <button
          key={e}
          type="button"
          className="reaction-picker-item"
          data-test={`reaction-picker-${e}`}
          aria-label={`React with ${e}`}
          onClick={() => {
            onPick(e)
          }}
        >
          {e}
        </button>
      ))}
    </div>
  )
}

interface Props {
  reactions: AggregatedReaction[] | undefined
  viewer: string
  onToggle: (emoji: string) => void
}

function Reactions({ reactions, viewer, onToggle }: Props): React.JSX.Element | null {
  const [pickerOpen, setPickerOpen] = useState<boolean>(false)
  const addButtonRef = useRef<HTMLButtonElement | null>(null)
  const pills = reactions ?? []

  if (pills.length === 0 && !pickerOpen) {
    return null
  }

  return (
    <div className="message-reactions" data-test="message-reactions">
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
            <span aria-hidden="true">{r.emoji}</span>
            <small>{r.count}</small>
          </button>
        )
      })}
      <span className="reaction-add-anchor">
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
      </span>
    </div>
  )
}

interface AddOnlyProps {
  onToggle: (emoji: string) => void
}

/**
 * Compact picker-only affordance for messages with no existing
 * reactions. Shown on message hover so the viewer can add the first
 * reaction without a full Reactions container reflowing the row.
 */
export function ReactionAddButton({ onToggle }: AddOnlyProps): React.JSX.Element {
  const [open, setOpen] = useState<boolean>(false)
  const buttonRef = useRef<HTMLButtonElement | null>(null)
  return (
    <span className="reaction-add-container">
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
    </span>
  )
}

export default Reactions
