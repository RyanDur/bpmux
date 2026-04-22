import { useState } from 'react'
import type { AggregatedReaction } from '../hooks/useReactions'
import './Reactions.css'

// Minimal curated emoji palette (#340). Full Slack-style picker with
// search over the entire Unicode emoji set is out-of-scope for v1;
// these 6 cover the common "ack / +1 / celebrate / ship it" signals
// that reactions are meant to carry in the first place. A richer
// picker can be added as a follow-up when users exhaust the palette.
export const EMOJI_PALETTE = ['👍', '🎉', '❤️', '🚀', '👀', '❌'] as const

interface Props {
  reactions: AggregatedReaction[] | undefined
  viewer: string
  onToggle: (emoji: string) => void
}

function Reactions({ reactions, viewer, onToggle }: Props): React.JSX.Element | null {
  const [pickerOpen, setPickerOpen] = useState<boolean>(false)
  const pills = reactions ?? []

  if (pills.length === 0 && !pickerOpen) {
    // Nothing rendered — caller decides whether to show the +emoji
    // button via its own hover-reveal. Returning null avoids layout
    // space when there's no reaction state.
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
      {pickerOpen && (
        <div
          className="reaction-picker"
          role="menu"
          aria-label="Pick a reaction"
          data-test="reaction-picker"
        >
          {EMOJI_PALETTE.map((e) => (
            <button
              key={e}
              type="button"
              className="reaction-picker-item"
              data-test={`reaction-picker-${e}`}
              aria-label={`React with ${e}`}
              onClick={() => {
                onToggle(e)
                setPickerOpen(false)
              }}
            >
              {e}
            </button>
          ))}
        </div>
      )}
      <button
        type="button"
        className="reaction-add"
        data-test="reaction-add"
        aria-label={pickerOpen ? 'Close reaction picker' : 'Add reaction'}
        title={pickerOpen ? 'Close' : 'Add reaction'}
        onClick={() => setPickerOpen((prev) => !prev)}
      >
        {pickerOpen ? '✕' : '+😊'}
      </button>
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
  return (
    <span className="reaction-add-container">
      <button
        type="button"
        className="reaction-add"
        data-test="reaction-add-first"
        aria-label={open ? 'Close reaction picker' : 'Add reaction'}
        title={open ? 'Close' : 'Add reaction'}
        onClick={() => setOpen((prev) => !prev)}
      >
        {open ? '✕' : '+😊'}
      </button>
      {open && (
        <div
          className="reaction-picker"
          role="menu"
          aria-label="Pick a reaction"
          data-test="reaction-picker"
        >
          {EMOJI_PALETTE.map((e) => (
            <button
              key={e}
              type="button"
              className="reaction-picker-item"
              data-test={`reaction-picker-${e}`}
              aria-label={`React with ${e}`}
              onClick={() => {
                onToggle(e)
                setOpen(false)
              }}
            >
              {e}
            </button>
          ))}
        </div>
      )}
    </span>
  )
}

export default Reactions
