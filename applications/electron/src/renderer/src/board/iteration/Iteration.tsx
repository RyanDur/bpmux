import { useIterationCollapse } from './useIterationCollapse'
import { useCollapsibleSectionsStorage } from '../useCollapsibleSectionsStorage'
import { RENDERER_IDENTITY } from '../../state'
import { findCurrentIteration } from './currentIteration'
import type { Story } from '../types'
import type { BoardIterationData } from './selectIterations'
import { type IterationDragPayload } from '../iterationMembers'
import { SortableList } from '../drag/SortableList'
import { ITERATION_ITSELF_DRAG } from '../drag/sortableDragConfigs'
import { useIterations } from './useIterations'
import { useIterationMembers } from './useIterationMembers'
import { useIterationReorder } from './useIterationReorder'
import IterationRow from './IterationRow'
import UnscheduledList from './UnscheduledList'

type Props = {
  stories: Story[]
  onOpen: (n: number) => void
  onReorderAnnounce?: (text: string) => void
}

const UNSCHEDULED_KEY = 'unscheduled'

type IterationSectionId = 'active-planned' | 'closed'

const ITERATION_SECTIONS_STORAGE_PREFIX = 'bpmux-iteration-section-open'
const ITERATION_SECTION_DEFAULTS: Record<IterationSectionId, boolean> = {
  'active-planned': true,
  closed: false
}

const Iteration = ({ stories, onOpen, onReorderAnnounce }: Props): React.JSX.Element | null => {
  const collapse = useIterationCollapse(RENDERER_IDENTITY)
  const iterations = useIterations()
  const { sortedLoose, getMembersFor, setPending } = useIterationMembers(stories, iterations)
  const sectionStorage = useCollapsibleSectionsStorage<IterationSectionId>(
    ITERATION_SECTIONS_STORAGE_PREFIX,
    RENDERER_IDENTITY,
    ITERATION_SECTION_DEFAULTS
  )

  const activePlannedIterations = iterations.filter((it) => !it.complete)
  const closedIterations = iterations.filter((it) => it.complete)

  const { reorderIterationByDrag, reorderIterationByKeyboard } = useIterationReorder(
    activePlannedIterations,
    onReorderAnnounce
  )

  const sectionOpen = sectionStorage.state
  const toggleSection = (id: IterationSectionId, next: boolean): void => {
    if (sectionOpen[id] === next) return
    sectionStorage.setState({ ...sectionOpen, [id]: next })
  }
  if (iterations.length === 0 && sortedLoose.length === 0) return null
  const currentIterationId = findCurrentIteration(iterations)?.id ?? null

  const activePlannedCount = activePlannedIterations.length
  return (
    <section className="board-iterations" data-test="board-iterations">
      <details
        className="board-iterations-section board-iterations-section--active"
        data-test="board-iterations-section-active-planned"
        aria-label="Active + Planned iterations"
        open={sectionOpen['active-planned']}
        onToggle={(event) => toggleSection('active-planned', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary disclosure-summary"
          data-test="board-iterations-section-active-planned-summary"
        >
          <i className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Active + Planned</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-active-planned-count"
          >
            ({activePlannedCount})
          </small>
        </summary>
        <SortableList<IterationDragPayload, BoardIterationData>
          items={activePlannedIterations}
          keyOf={(it) => it.id}
          rowKey={(it) => it.id}
          config={ITERATION_ITSELF_DRAG}
          buildPayload={(it) => ({
            iterationId: it.id
          })}
          onReorder={reorderIterationByDrag}
          ulClassName="board-iterations-list"
          renderRow={(it, ctx) => (
            <IterationRow
              iteration={it}
              collapse={collapse}
              currentIterationId={currentIterationId}
              getMembersFor={getMembersFor}
              setPending={setPending}
              onOpen={onOpen}
              onReorderAnnounce={onReorderAnnounce}
              onReorderByKeyboard={reorderIterationByKeyboard}
              drag={ctx}
            />
          )}
        />
      </details>
      <details
        className="board-iterations-section board-iterations-section--closed"
        data-test="board-iterations-section-closed"
        aria-label="Closed iterations"
        open={sectionOpen.closed}
        onToggle={(event) => toggleSection('closed', event.currentTarget.open)}
      >
        <summary
          className="board-iterations-section-summary disclosure-summary"
          data-test="board-iterations-section-closed-summary"
        >
          <i className="board-iterations-section-chevron" aria-hidden="true" />
          <b className="board-iterations-section-label">Closed</b>
          <small
            className="board-iterations-section-count"
            data-test="board-iterations-section-closed-count"
          >
            ({closedIterations.length})
          </small>
        </summary>
        <ul className="board-iterations-list">
          {closedIterations.map((it) => (
            <IterationRow
              key={it.id}
              iteration={it}
              collapse={collapse}
              currentIterationId={currentIterationId}
              getMembersFor={getMembersFor}
              setPending={setPending}
              onOpen={onOpen}
              onReorderAnnounce={onReorderAnnounce}
              onReorderByKeyboard={reorderIterationByKeyboard}
            />
          ))}
        </ul>
      </details>
      <UnscheduledList
        stories={sortedLoose}
        open={collapse.isOpen(UNSCHEDULED_KEY, true)}
        onToggle={(o) => collapse.setOpen(UNSCHEDULED_KEY, o)}
        onOpen={onOpen}
      />
    </section>
  )
}

export default Iteration
