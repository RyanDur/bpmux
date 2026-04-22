import './CiGlyph.css'

// Phase-1a titlebar CI glyph placeholder. The real CI hook is a
// follow-up — a text placeholder is fine per the handoff. Lives in
// its own feature folder so future CI state wiring (pass/fail/pending
// colors, drill-in popover) has a home that doesn't sprawl the
// titlebar component.
function CiGlyph(): React.JSX.Element {
  return (
    <span className="ci-glyph" title="CI status (coming soon)">
      CI ?
    </span>
  )
}

export default CiGlyph
