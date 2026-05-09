import type { IterationMarker as IterationMarkerData } from '../hooks/useIterationMarkers'

interface Props {
  marker: IterationMarkerData
  variant: 'next' | 'future'
}

const VARIANT_LABEL: Record<Props['variant'], string> = {
  next: 'next iteration',
  future: 'future iteration'
}

function IterationMarker({ marker, variant }: Props): React.JSX.Element {
  return (
    <li
      className={`board-iteration-marker board-iteration-marker--${variant}`}
      data-test={`iteration-marker-${marker.id}`}
      data-variant={variant}
      role="separator"
      aria-label={`${VARIANT_LABEL[variant]}: ${marker.theme_name}`}
    >
      <span className="board-iteration-marker-glyph" aria-hidden="true">
        ★
      </span>
      <b className="board-iteration-marker-theme">{marker.theme_name}</b>
      <small className="board-iteration-marker-label">{VARIANT_LABEL[variant]}</small>
    </li>
  )
}

export default IterationMarker
