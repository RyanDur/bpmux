import type { JSX } from 'react'

interface ReworkGlyphProps {
  reworkComment?: string | null
}

// #1146: surfaces the ↩ glyph at the column-surface when a story's
// most-recent state-transition is backward (server populates
// rework_comment via replay_transitions + most_recent_backward_transition
// in story_to_board_json). Tooltip carries the backward-edge comment
// so dev sees the rework reason on hover/focus without opening the
// story.
export const ReworkGlyph = ({ reworkComment }: ReworkGlyphProps): JSX.Element | null => {
  if (!reworkComment) return null
  return (
    <span
      className="board-story-rework-glyph"
      title={reworkComment}
      aria-label={`Rework: ${reworkComment}`}
    >
      ↩
    </span>
  )
}
