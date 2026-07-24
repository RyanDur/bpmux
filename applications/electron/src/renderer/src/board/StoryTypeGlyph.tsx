import type { StoryType } from './types'

type Props = {
  type: StoryType
}

const StoryTypeGlyph = ({ type }: Props): React.JSX.Element => {
  const label = typeLabel(type)
  return (
    <svg
      className={`story-glyph story-glyph--${type}`}
      width="16"
      height="16"
      viewBox="0 0 16 16"
      aria-label={label}
      role="img"
      focusable="false"
    >
      {renderPath(type)}
    </svg>
  )
}

const typeLabel = (type: StoryType): string => {
  switch (type) {
    case 'feature':
      return 'Feature'
    case 'bug':
      return 'Bug'
    case 'chore':
      return 'Chore'
    case 'epic':
      return 'Epic'
    case 'iteration':
      return 'Iteration'
    default:
      return 'Story'
  }
}

const renderPath = (type: StoryType): React.JSX.Element => {
  switch (type) {
    case 'feature':
      return (
        <path
          d="M8 1.5l1.4 4.1 4.1 1.4-4.1 1.4L8 12.5 6.6 8.4 2.5 7l4.1-1.4z"
          fill="currentColor"
        />
      )
    case 'bug':
      return (
        <g fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
          <path d="M5.5 3.2l1.2 1.6M10.5 3.2l-1.2 1.6" />
          <path
            d="M4.5 8.5a3.5 3.5 0 0 1 7 0v2.5a3.5 3.5 0 0 1-7 0z"
            fill="currentColor"
            stroke="currentColor"
          />
          <path d="M2 8h2.5M11.5 8H14M2.5 11.5l2-1M13.5 11.5l-2-1" />
        </g>
      )
    case 'chore':
      return (
        <g fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round">
          <path d="M8 2.3v1.8M8 11.9v1.8M2.3 8h1.8M11.9 8h1.8M4.2 4.2l1.3 1.3M10.5 10.5l1.3 1.3M4.2 11.8l1.3-1.3M10.5 5.5l1.3-1.3" />
          <circle cx="8" cy="8" r="2.6" fill="currentColor" stroke="currentColor" />
        </g>
      )
    case 'epic':
      return (
        <g fill="currentColor">
          <rect x="3" y="11.5" width="10" height="2" rx="0.5" />
          <rect x="4.5" y="8" width="7" height="2" rx="0.5" />
          <rect x="6" y="4.5" width="4" height="2" rx="0.5" />
          <rect x="7.25" y="1.5" width="1.5" height="2" rx="0.5" />
        </g>
      )
    case 'iteration':
      return (
        <g fill="currentColor">
          <rect x="3.5" y="2" width="1" height="12" rx="0.3" />
          <path d="M4.5 2.5h7l-1.6 2.2 1.6 2.2h-7z" />
        </g>
      )
    default:
      return <circle cx="8" cy="8" r="2" fill="currentColor" />
  }
}

export default StoryTypeGlyph
