import { GripIcon } from './GripIcon'
import { STORY_GRIP_TEST_ID } from './testIds'
import { type GripDragProps } from './useDragReorder'

export interface DragGripProps {
  storyNumber: number
  gripDragProps: GripDragProps
}

export const DragGrip = ({ storyNumber, gripDragProps }: DragGripProps): React.JSX.Element => (
  <button
    type="button"
    className="board-story-grip"
    data-test={STORY_GRIP_TEST_ID}
    aria-label={`Drag to reorder #${storyNumber}`}
    tabIndex={-1}
    {...gripDragProps}
  >
    <GripIcon />
  </button>
)
