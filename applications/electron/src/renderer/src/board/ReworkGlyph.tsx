import type { JSX } from 'react'

interface ReworkGlyphProps {
  reworkComment?: string | null
}

export const ReworkGlyph = ({ reworkComment }: ReworkGlyphProps): JSX.Element | null => {
  if (!reworkComment) return null
  return (
    <i
      className="board-story-rework-glyph"
      title={reworkComment}
      aria-label={`Rework: ${reworkComment}`}
    >
      ↩
    </i>
  )
}
