import { useEffect, useState } from 'react'
import IterationMarker from './IterationMarker'
import StoryRow from './StoryRow'
import type { BoardStory } from './types'
import type { IterationMarker as IterationMarkerData } from '../hooks/useIterationMarkers'

interface IterationGroupProps {
  stories: BoardStory[]
  marker: IterationMarkerData
  variant: 'next' | 'future'
  viewerIdentity: string
  selectedNumber: number | null
  draggable: boolean
  draggingNumber: number | null
  baseIndex: number
  total: number
  dropAt: number | null
  epicTitleByNumber: Map<number, string>
  onOpen: (n: number) => void
  onOpenTimeline?: (n: number) => void
  onSelect: (n: number) => void
  onKeyboardReorder?: (storyNumber: number, direction: 'up' | 'down') => void
  onRowDragOver: (index: number) => (event: React.DragEvent<HTMLLIElement>) => void
  onDragStartRow: (storyNumber: number) => void
  onDragEndRow: () => void
}

const collapseKey = (viewerIdentity: string, markerId: number): string =>
  `bpmux-board-marker-collapse:${viewerIdentity}:${markerId}`

const readCollapsed = (viewerIdentity: string, markerId: number): boolean => {
  if (typeof window === 'undefined') return false
  const raw = window.localStorage.getItem(collapseKey(viewerIdentity, markerId))
  return raw === 'true'
}

const writeCollapsed = (viewerIdentity: string, markerId: number, collapsed: boolean): void => {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(collapseKey(viewerIdentity, markerId), String(collapsed))
}

export const VARIANT_LABEL: Record<'next' | 'future', string> = {
  next: 'next iteration',
  future: 'future iteration'
}

export const summaryAriaLabel = (
  marker: IterationMarkerData,
  variant: 'next' | 'future',
  count: number
): string =>
  `${VARIANT_LABEL[variant]}: ${marker.theme_name}, ${count} ${count === 1 ? 'story' : 'stories'}`

const IterationGroup = ({
  stories,
  marker,
  variant,
  viewerIdentity,
  selectedNumber,
  draggable,
  draggingNumber,
  baseIndex,
  total,
  dropAt,
  epicTitleByNumber,
  onOpen,
  onOpenTimeline,
  onSelect,
  onKeyboardReorder,
  onRowDragOver,
  onDragStartRow,
  onDragEndRow
}: IterationGroupProps): React.JSX.Element => {
  const [collapsed, setCollapsed] = useState<boolean>(() =>
    readCollapsed(viewerIdentity, marker.id)
  )
  useEffect(() => {
    writeCollapsed(viewerIdentity, marker.id, collapsed)
  }, [viewerIdentity, marker.id, collapsed])

  const count = stories.length
  return (
    <details
      className={`board-iteration-group board-iteration-group--${variant}`}
      data-test={`iteration-group-${marker.id}`}
      open={!collapsed}
      onToggle={(event) => {
        const nextOpen = (event.currentTarget as HTMLDetailsElement).open
        setCollapsed(!nextOpen)
      }}
    >
      <summary
        className={`board-iteration-marker board-iteration-marker--${variant}`}
        data-test={`iteration-marker-${marker.id}`}
        data-variant={variant}
        aria-label={summaryAriaLabel(marker, variant, count)}
      >
        <span className="board-iteration-marker-glyph" aria-hidden="true">
          ★
        </span>
        <b className="board-iteration-marker-theme">{marker.theme_name}</b>
        <small className="board-iteration-marker-label">{VARIANT_LABEL[variant]}</small>
        <small className="board-iteration-marker-count" data-test={`iteration-marker-${marker.id}-count`}>
          {count} {count === 1 ? 'story' : 'stories'}
        </small>
      </summary>
      <ul className="board-story-list board-iteration-stories">
        {stories.map((story, localIdx) => {
          const i = baseIndex + localIdx
          return (
            <StoryRow
              key={story.number}
              story={story}
              parentEpicTitle={
                story.parent_epic !== undefined
                  ? epicTitleByNumber.get(story.parent_epic)
                  : undefined
              }
              selected={story.number === selectedNumber}
              draggable={draggable}
              dragging={draggingNumber === story.number}
              dropBefore={dropAt === i}
              dropAfterLast={dropAt === total && i === total - 1}
              position={i + 1}
              total={total}
              onKeyboardReorder={
                draggable && onKeyboardReorder !== undefined
                  ? (direction) => onKeyboardReorder(story.number, direction)
                  : undefined
              }
              onOpen={onOpen}
              onOpenTimeline={onOpenTimeline}
              onSelect={onSelect}
              onRowDragOver={onRowDragOver(i)}
              onDragStartRow={() => onDragStartRow(story.number)}
              onDragEndRow={onDragEndRow}
            />
          )
        })}
      </ul>
    </details>
  )
}

export default IterationGroup
export { IterationMarker }
